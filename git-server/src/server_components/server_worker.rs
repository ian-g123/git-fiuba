use std::{
    io::{self, Cursor, Write},
    net::TcpStream,
};

use git_lib::{
    command_errors::CommandError,
    file_compressor::compress,
    git_repository::GitRepository,
    join_paths,
    logger::Logger,
    objects,
    server_components::{
        packfile_object_type::{self, PackfileObjectType},
        pkt_strings::Pkt,
    },
    utils::aux::read_string_until,
};

pub struct ServerWorker {
    path: String,
    socket: TcpStream,
}

impl ServerWorker {
    pub fn new(path: String, stream: TcpStream) -> Self {
        Self {
            path,
            socket: stream,
        }
    }

    pub fn handle_connection(&mut self) -> Result<(), CommandError> {
        let Some(presentation) = String::read_pkt_format(&mut self.socket)? else {
            return Err(CommandError::ErrorReadingPkt);
        };
        let presentation_components: Vec<&str> = presentation.split("\0").collect();
        let command_and_repo_path = presentation_components[0];
        let (command, repo_path) = command_and_repo_path
            .split_once(" ")
            .ok_or(CommandError::ErrorReadingPkt)?;

        match command {
            "git-upload-pack" => {
                println!("git-upload-pack");
                self.git_upload_pack(&repo_path[1..])
            }
            "git-receive-pack" => {
                println!("git-receive-pack");
                todo!("git-receive-pack not implemented");
            }
            _ => {
                println!("command not found");
                todo!("command not found not implemented");
            }
        }
    }

    fn git_upload_pack(&mut self, repo_path: &str) -> Result<(), CommandError> {
        println!("git-upload-pack method");
        let mut stdout = io::stdout();
        let joint_path = join_paths!(self.path, repo_path).ok_or(CommandError::Io {
            message: format!(
                "No se pudo unir el path {} con el path {}",
                self.path, repo_path
            ),
            error: "".to_string(),
        })?;
        println!("joint path: {}", joint_path);
        let mut repo = GitRepository::open(&joint_path, &mut stdout).map_err(|error| {
            CommandError::Io {
                message: format!("No se pudo abrir el repositorio {}.\n Tal vez no sea el path correcto o no tengas acceso.", joint_path),
                error: error.to_string(),
            }
        })?;
        let head_branch_name = repo.get_head_branch_name()?;
        let local_branches = repo.local_branches()?;
        println!("head branch: {}", head_branch_name);
        let head_branch_hash = local_branches.get(&head_branch_name).unwrap().clone();
        let mut sorted_branches = local_branches
            .into_iter()
            .collect::<Vec<(String, String)>>();
        sorted_branches.sort_unstable();
        println!("local branches: {:?}", sorted_branches);
        self.send("version 1\n")?;
        self.send(&format!("{} HEAD\0\n", head_branch_hash,))?;
        for (branch_name, branch_hash) in sorted_branches {
            self.send(&format!("{} refs/heads{}\n", branch_hash, branch_name))?;
        }
        self.write_string_to_socket("0000")?;
        let (want_lines, have_lines) = self.read_wants_and_haves()?;
        let packfile = self.build_pack_file(&mut repo, want_lines, have_lines)?;
        self.socket.write_all(&packfile);
        Ok(())
    }

    fn read_wants_and_haves(&mut self) -> Result<(Vec<String>, Vec<String>), CommandError> {
        let want_and_have_lines = get_response_until(&mut self.socket, "done\n")?;
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
        // self.write_to_socket(line.as_bytes());
        let message = line.as_bytes();
        self.socket
            .write_all(message)
            .map_err(|error| CommandError::SendingMessage(error.to_string()))?;
        Ok(())
    }

    fn build_pack_file(
        &self,
        repo: &mut GitRepository<'_>,
        want_lines: Vec<String>,
        have_lines: Vec<String>,
    ) -> Result<Vec<u8>, CommandError> {
        println!("want_lines: {:?}", want_lines);
        println!("have_lines: {:?}", have_lines);
        if !have_lines.is_empty() {
            todo!("TODO implementar have_lines")
        }
        let mut pack_file = Vec::<u8>::new();
        let mut objects = Vec::<Vec<u8>>::new();
        for want_line in want_lines {
            let (_, hash_str) =
                want_line
                    .split_once(' ')
                    .ok_or(CommandError::PackageNegotiationError(
                        "No se pudo leer la línea want".to_string(),
                    ))?;
            let (path, object_content) =
                repo.db()?.read_file(hash_str, &mut Logger::new_dummy())?;
            objects.push(object_content);
        }
        pack_file.extend("PACK".as_bytes());
        let version = (2 as u32).to_le_bytes();
        println!("Version bytes: {:?}", version);
        pack_file.extend(version);
        let num_objects = (objects.len() as u32).to_le_bytes();
        println!("Num objects bytes: {:?}", num_objects);
        pack_file.extend(num_objects);
        for object_content in objects {
            let object_size = object_content.len();
            let compressed_object = compress(&object_content)?;
            let mut cursor = Cursor::new(object_content);
            let type_str = read_string_until(&mut cursor, ' ')?;
            let pf_type = PackfileObjectType::from_str(type_str.as_str())?;

            // Size encoding
            // This document uses the following "size encoding" of non-negative integers: From each byte, the seven least
            // significant bits are used to form the resulting integer. As long as the most significant bit is 1, this
            // process continues; the byte with MSB 0 provides the last seven bits. The seven-bit chunks are concatenated.
            // Later values are more significant.
            // This size encoding should not be confused with the "offset encoding", which is also used in this document.

            let mut len_temp = object_size;
            let first_four = (len_temp & 0b00001111) as u8;
            len_temp >>= 4;
            let mut len_bytes: Vec<u8> = Vec::new();
            loop {
                let mut byte = (len_temp & 0b01111111) as u8;
                len_temp >>= 7;
                if len_temp == 0 {
                    len_bytes.push(byte);
                    break;
                }
                byte |= 0b10000000;
                len_bytes.push(byte);
            }

            let type_and_len_byte = (pf_type as u8) << 4
                | first_four
                | if len_bytes.is_empty() { 0 } else { 0b10000000 };

            pack_file.push(type_and_len_byte);
            pack_file.extend(len_bytes);
            pack_file.extend(compressed_object);
        }
        todo!()
    }
}

fn get_response_until(
    socket: &mut TcpStream,
    stop_line: &str,
) -> Result<Vec<Vec<String>>, CommandError> {
    let mut lines_groups = Vec::<Vec<String>>::new();
    let mut current_lines_group = Vec::<String>::new();
    loop {
        match String::read_pkt_format(socket)? {
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

fn get_response(mut socket: &TcpStream) -> Result<Vec<String>, CommandError> {
    let mut lines = Vec::<String>::new();
    loop {
        match String::read_pkt_format(&mut socket)? {
            Some(line) => {
                lines.push(line);
            }
            None => break,
        }
    }
    Ok(lines)
}
