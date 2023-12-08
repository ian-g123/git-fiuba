use std::{
    collections::{HashMap, HashSet},
    io::{self, Cursor, Write},
    net::TcpStream,
};

use git_lib::{
    command_errors::CommandError,
    git_repository::GitRepository,
    join_paths,
    logger::Logger,
    logger_sender::LoggerSender,
    objects::{blob::Blob, commit_object::CommitObject, git_object::GitObjectTrait, tree::Tree},
    server_components::{
        history_analyzer::rebuild_commits_tree,
        packfile_functions::{make_packfile, read_objects_from_packfile},
        packfile_object_type::PackfileObjectType,
        pkt_strings::Pkt,
    },
    utils::super_string::u8_vec_to_hex_string,
};

pub struct ServerWorker {
    path: String,
    socket: TcpStream,
    process_id: String,
    thread_id: String,
    logger_sender: LoggerSender,
}

impl ServerWorker {
    pub fn new(path: String, stream: TcpStream, logger_sender: LoggerSender) -> Self {
        let process_id = format!("{:?}", std::process::id());
        let thread_id = format!("{:?}", std::thread::current().id());
        Self {
            path,
            socket: stream,
            process_id,
            thread_id,
            logger_sender,
        }
    }

    fn log(&mut self, message: &str) {
        let time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        self.logger_sender.log(&format!(
            "[{}:{}] {}: {}",
            self.process_id, self.thread_id, time, message
        ));
    }

    pub fn handle_connection(&mut self) {
        self.log("New connection");
        match self.handle_connection_priv() {
            Ok(_) => self.log("Connection handled successfully"),
            Err(error) => {
                self.log(&format!("❌ Error: {}", error));
                eprintln!("{error}")
            }
        }
    }

    fn handle_connection_priv(&mut self) -> Result<(), CommandError> {
        let Some(presentation) = self.read_tpk()? else {
            return Err(CommandError::ErrorReadingPktVerbose(
                "handle_connection_priv leyó flush-pkt".to_string(),
            ));
        };
        let presentation_components: Vec<&str> = presentation.split('\0').collect();
        let command_and_repo_path = presentation_components[0];
        let (command, repo_path) =
            command_and_repo_path
                .split_once(' ')
                .ok_or(CommandError::ErrorReadingPktVerbose(format!(
                    "Error al leer command_and_repo_path: {}",
                    command_and_repo_path
                )))?;

        match command {
            "git-upload-pack" => self.git_upload_pack(&repo_path[1..]),
            "git-receive-pack" => self.git_receive_pack(&repo_path[1..]),
            _ => {
                self.log("command not found");
                todo!("command not found not implemented");
            }
        }
    }

    fn git_upload_pack(&mut self, repo_relative_path: &str) -> Result<(), CommandError> {
        self.log("==========git-upload-pack method==========");
        let mut stdout = io::stdout();
        let repo_path = self.repo_path(repo_relative_path)?;
        let mut repo = GitRepository::open(&repo_path, &mut stdout).map_err(|error| {
            if let Err(error_send) =self.write_string_to_socket("0000") {
                return error_send
            }
            CommandError::Io {
                message: format!("No se pudo abrir el repositorio {}.\n Tal vez no sea el path correcto o no tengas acceso.", repo_path),
                error: error.to_string(),
            }
        })?;

        let head_branch_name = repo.get_head_branch_name()?;
        let local_branches_refs = repo.local_branches_refs()?;
        self.log(&format!("local_branches: {:?}", local_branches_refs));
        self.log(&format!("head_branch_name: {:?}", head_branch_name));
        let head_branch_ref_str = match local_branches_refs.get(&head_branch_name) {
            Some(head_branch_hash) => head_branch_hash,
            None => "0000000000000000000000000000000000000000",
        };
        let mut sorted_branches = local_branches_refs
            .clone()
            .into_iter()
            .collect::<Vec<(String, String)>>();
        sorted_branches.sort_unstable();
        self.send("version 1\n")?;
        self.send(&format!("{} HEAD\0\n", head_branch_ref_str))?;
        for (branch_name, branch_hash) in sorted_branches {
            self.send(&format!("{} refs/heads/{}\n", branch_hash, branch_name))?;
        }
        self.write_string_to_socket("0000")?;

        let (want_lines, have_lines) = self.read_wants_and_haves()?;
        self.send("NAK\n")?;
        let packfile = self.build_pack_file(&mut repo, want_lines, have_lines)?;

        self.socket.write_all(&packfile).map_err(|error| {
            CommandError::SendingMessage(format!("Error enviando packfile: {}", error))
        })?;
        Ok(())
    }

    fn git_receive_pack(&mut self, repo_relative_path: &str) -> Result<(), CommandError> {
        self.log("==========git-receive-pack method==========");
        let mut stdout = io::stdout();
        let repo_path = self.repo_path(repo_relative_path)?;
        let mut repo = GitRepository::open(&repo_path, &mut stdout).map_err(|error| {
            CommandError::Io {
                message: format!("No se pudo abrir el repositorio {}.\n Tal vez no sea el path correcto o no tengas acceso.", repo_path),
                error: error.to_string(),
            }
        })?;

        self.send("version 1")?;

        let local_branches_refs: HashMap<String, String> = repo.local_branches_refs()?;
        if local_branches_refs.is_empty() {
            let head_name = repo.get_head_branch_name()?;
            self.send(&format!(
                "0000000000000000000000000000000000000000 refs/heads/{}\0\n",
                head_name
            ))?;
        } else {
            let mut sorted_branches = local_branches_refs
                .clone()
                .into_iter()
                .collect::<Vec<(String, String)>>();
            sorted_branches.sort_unstable();
            let (first_branch_name, first_branch_hash) = sorted_branches.remove(0);

            self.send(&format!(
                "{} refs/heads/{}\0\n",
                first_branch_hash, first_branch_name
            ))?;
            for (branch_name, branch_hash) in sorted_branches {
                self.send(&format!("{} refs/heads/{}\n", branch_hash, branch_name))?;
            }
        }
        self.write_string_to_socket("0000")?;

        let ref_update_map = self.read_ref_update_map()?;

        let objects = read_objects_from_packfile(&mut self.socket, &repo.db()?, repo.logger())?;
        let objects_map = repo.save_objects_from_packfile(objects)?;
        let mut status = HashMap::<String, Option<String>>::new();

        self.send("unpack ok\n")?;
        for (branch_path, (old_ref, new_ref)) in ref_update_map {
            let branch_name = branch_path[11..].to_string();
            let local_branch_hash =
                if let Some(local_branch_hash) = local_branches_refs.get(&branch_name) {
                    local_branch_hash
                } else {
                    repo.create_branch(&branch_name, &new_ref, None)?;
                    &new_ref
                };

            if local_branch_hash != &old_ref {
                status.insert(
                    branch_name.to_string(),
                    Some("non-fast-forward".to_string()),
                );
            } else if check_commits_between(
                &objects_map,
                &old_ref,
                &new_ref,
                &mut Logger::new_dummy(),
            )? {
                self.log(&format!(
                    "Actualizando rama {} de {} a {}",
                    branch_name, old_ref, new_ref
                ));
                status.insert(branch_name.to_string(), None);
                repo.update_branch_ref(&new_ref, &branch_name)?;
            } else {
                self.log(
                    "No se pudo hacer fast-forward porque no se encontraron todos los commits",
                );
                status.insert(branch_name, Some("non-fast-forward".to_string()));
            }
        }

        Ok(())
    }

    fn repo_path(&mut self, relative: &str) -> Result<String, CommandError> {
        let joint_path = join_paths!(self.path, relative).ok_or(CommandError::Io {
            message: format!(
                "No se pudo unir el path {} con el path {}",
                self.path, relative
            ),
            error: "".to_string(),
        })?;
        Ok(joint_path)
    }

    fn read_wants_and_haves(&mut self) -> Result<(Vec<String>, Vec<String>), CommandError> {
        let want_and_have_lines = self.get_responses_until("done\n")?;
        let want_lines = want_and_have_lines
            .get(0)
            .ok_or(CommandError::PackageNegotiationError(
                "No se recibieron líneas want".to_string(),
            ))?
            .to_owned();
        let have_lines = match want_and_have_lines.get(1) {
            Some(lines) => lines.to_owned(),
            None => vec![],
        };
        Ok((want_lines, have_lines))
    }

    /// envía un mensaje al servidor
    fn send(&mut self, line: &str) -> Result<(), CommandError> {
        let line = line.to_string().to_pkt_format();
        self.write_string_to_socket(&line)?;
        Ok(())
    }

    fn write_string_to_socket(&mut self, line: &str) -> Result<(), CommandError> {
        self.log(&format!("⏫: {:?}", line));
        let message = line.as_bytes();
        self.socket
            .write_all(message)
            .map_err(|error| CommandError::SendingMessage(error.to_string()))?;
        Ok(())
    }

    fn build_pack_file(
        &mut self,
        repo: &mut GitRepository<'_>,
        want_lines: Vec<String>,
        have_lines: Vec<String>,
    ) -> Result<Vec<u8>, CommandError> {
        self.log("build_pack_file");
        let haves: Result<HashSet<String>, CommandError> = have_lines
            .into_iter()
            .map(|have_line| {
                let (_, hash_str) =
                    have_line
                        .split_once(' ')
                        .ok_or(CommandError::PackageNegotiationError(
                            "No se pudo leer la línea have".to_string(),
                        ))?;
                Ok(hash_str.trim().to_string())
            })
            .collect();

        let haves = haves?;

        let wants: Result<HashSet<String>, CommandError> = want_lines
            .into_iter()
            .map(|want_line| {
                let (_, hash_str) =
                    want_line
                        .split_once(' ')
                        .ok_or(CommandError::PackageNegotiationError(
                            "No se pudo leer la línea want".to_string(),
                        ))?;
                Ok(hash_str.trim().to_string())
            })
            .collect();

        let wants = wants?;

        self.log(&format!("haves: {:?}", haves));
        self.log(&format!("wants: {:?}", wants));
        let mut commits_map = HashMap::<String, (CommitObject, usize, usize)>::new();
        for want in wants {
            self.log(&format!("Painting commits upto: {}", want));
            rebuild_commits_tree(
                &repo.db()?,
                &want,
                &mut commits_map,
                &haves,
                true,
                repo.logger(),
                0,
            )?;
            self.log(&format!("commits_map: {:?}", commits_map));
        }
        self.log(&format!("after rebulding: {:?}", commits_map));

        for have in haves {
            self.log(&format!("removing: {:?}", have));
            commits_map.remove(&have);
            self.log(&format!("commits_map: {:?}", commits_map));
        }
        self.log(&format!("commits_map: {:?}", commits_map));
        self.log("╔==========");
        self.log("║ Packfile summary");
        for (hash, (commit, _, _)) in &commits_map {
            self.log(&format!("║ {}: {}", hash, commit.get_message()));
            let mut hash_stack = Vec::<Tree>::new();
            let value = commit
                .get_tree()
                .ok_or(CommandError::CheckingCommitsBetweenError(
                    "No se pudo leer el commit".to_string(),
                ))?
                .to_owned();
            hash_stack.push(value);
            while let Some(mut tree) = hash_stack.pop() {
                self.log(&format!("║     tree: {}", tree.get_hash_string()?));
                for (_, (_, object_opt)) in tree.get_objects() {
                    let Some(mut object) = object_opt else {
                        continue;
                    };
                    if let Some(subtree) = object.as_mut_tree() {
                        hash_stack.push(subtree.to_owned());
                    }
                    if let Some(blob) = object.as_mut_blob() {
                        self.log(&format!("║     blob: {}", blob.get_hash_string()?));
                    }
                }
            }
        }
        self.log("╚==========");

        make_packfile(commits_map)
    }

    fn read_ref_update_map(&mut self) -> Result<HashMap<String, (String, String)>, CommandError> {
        let map_lines = self.get_response_until_flushpkt()?;
        let mut map = HashMap::<String, (String, String)>::new();
        let mut is_first = true;
        for map_line in map_lines {
            let parts = map_line.split(' ').collect::<Vec<&str>>();
            let old_ref = parts[0].to_string();
            let new_ref = parts[1].to_string();
            let mut branch_name = parts[2].to_string();

            if is_first {
                branch_name.pop();
                map.insert(
                    branch_name.trim().to_string(),
                    (old_ref.trim().to_string(), new_ref.trim().to_string()),
                );
                is_first = false;
            } else {
                map.insert(
                    branch_name.trim().to_string(),
                    (old_ref.trim().to_string(), new_ref.trim().to_string()),
                );
            }
        }
        Ok(map)
    }

    fn get_responses_until(&mut self, stop_line: &str) -> Result<Vec<Vec<String>>, CommandError> {
        let mut lines_groups = Vec::<Vec<String>>::new();
        let mut current_lines_group = Vec::<String>::new();
        loop {
            match self.read_tpk()? {
                Some(line) => {
                    if line == stop_line {
                        lines_groups.push(current_lines_group);
                        break;
                    }
                    current_lines_group.push(line);
                }
                None => {
                    lines_groups.push(current_lines_group);
                    current_lines_group = Vec::<String>::new();
                }
            }
        }

        Ok(lines_groups)
    }

    fn get_response_until_flushpkt(&mut self) -> Result<Vec<String>, CommandError> {
        let mut response = Vec::<String>::new();
        while let Some(line) = self.read_tpk()? {
            if line == "0000" {
                break;
            }
            response.push(line);
        }

        Ok(response)
    }

    fn read_tpk(&mut self) -> Result<Option<String>, CommandError> {
        let line = String::read_pkt_format(&mut self.socket)?;
        if let Some(s_line) = line.to_owned() {
            self.log(&format!("⬇️: {:?}", s_line));
        }
        Ok(line)
    }
}

fn check_commits_between(
    objects_map: &HashMap<String, (PackfileObjectType, usize, Vec<u8>)>,
    old_ref: &str,
    new_ref: &str,
    logger: &mut Logger,
) -> Result<bool, CommandError> {
    if new_ref == old_ref {
        return Ok(true);
    }
    let Some((object_type, _, object_content)) = objects_map.get(new_ref) else {
        return Ok(false);
    };

    match object_type {
        PackfileObjectType::Commit => {
            let mut cursor = Cursor::new(object_content);
            let git_object_trait = &mut CommitObject::read_from(None, &mut cursor, logger, None)?;
            let commit_object = git_object_trait.as_mut_commit().ok_or(
                CommandError::CheckingCommitsBetweenError("No se pudo leer el commit".to_string()),
            )?;
            let tree_hash = commit_object.get_tree_hash_string();
            if !contains_all_elements(objects_map, &tree_hash)? {
                return Ok(false);
            }
            let parents = commit_object.get_parents();
            for parent in parents {
                if !check_commits_between(objects_map, old_ref, &parent, logger)? {
                    return Ok(false);
                };
            }
            Ok(true)
        }
        _ => {
            panic!("No debería pasar esto")
        }
    }
}

fn contains_all_elements(
    objects_map: &HashMap<String, (PackfileObjectType, usize, Vec<u8>)>,
    hash: &str,
) -> Result<bool, CommandError> {
    let Some((object_type, object_len, object_content)) = objects_map.get(hash) else {
        return Ok(false);
    };

    match object_type {
        PackfileObjectType::Commit => {
            return Err(CommandError::ObjectNotTree);
        }
        PackfileObjectType::Tree => {
            let mut cursor = Cursor::new(object_content);
            let tree_object = &mut Tree::read_from(
                None,
                &mut cursor,
                object_len.to_owned(),
                "",
                "",
                &mut Logger::new_dummy(),
            )?;
            let Some(tree) = tree_object.as_mut_tree() else {
                return Ok(false);
            };
            let objects_hashmap = tree.get_objects();
            for (_, (object_hash, _)) in objects_hashmap {
                let object_hash_str = u8_vec_to_hex_string(&object_hash);
                if !contains_all_elements(objects_map, &object_hash_str)? {
                    return Ok(false);
                }
            }
        }
        PackfileObjectType::Blob => {
            let mut cursor = Cursor::new(object_content);
            let blob_object = &mut Blob::read_from(
                &mut cursor,
                object_len.to_owned(),
                "",
                "",
                &mut Logger::new_dummy(),
            )?;
            if blob_object.as_mut_blob().is_none() {
                return Ok(false);
            };
            return Ok(true);
        }

        _ => {
            return Ok(false);
        }
    }

    Ok(true)
}
