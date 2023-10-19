use std::{
    fs::{self, File, OpenOptions},
    io::Read,
    io::Write,
};

use chrono::{DateTime, Local};

use crate::{
    commands::{
        add_components::add::{self, Add},
        branch_manager::{get_current_branch, get_last_commit},
        command::{Command, ConfigAdderFunction},
        command_errors::CommandError,
        config::Config,
        objects::{author::Author, commit_object::CommitObject, git_object::GitObject, tree::Tree},
        objects_database,
        stagin_area::{self, StagingArea},
    },
    logger::Logger,
};

pub struct Commit {
    all: bool,
    reuse_message: Option<String>,
    dry_run: bool,
    message: Option<String>,
    quiet: bool,
    files: Vec<String>,
}

impl Command for Commit {
    fn run_from(
        name: &str,
        args: &[String],
        stdin: &mut dyn Read,
        output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        if name != "commit" {
            return Err(CommandError::Name);
        }
        logger.log(&format!("committing {:?}", args));

        let instance = Commit::new_from(args)?;

        instance.run(stdin, output, logger)?;
        logger.log(&format!("commit {:?}", args));
        Ok(())
    }

    fn config_adders(&self) -> ConfigAdderFunction<Commit> {
        vec![
            Commit::add_all_config,
            Commit::add_dry_run_config,
            Commit::add_message_config,
            Commit::add_quiet_config,
            Commit::add_reuse_message_config,
            Commit::add_pathspec_config,
        ]
    }
}

impl Commit {
    fn new_from(args: &[String]) -> Result<Self, CommandError> {
        let mut commit = Self::new_default();
        commit.config(args)?;
        Ok(commit)
    }

    fn new_default() -> Self {
        Commit {
            all: false,
            reuse_message: None,
            dry_run: false,
            message: None,
            quiet: false,
            files: Vec::new(),
        }
    }

    fn check_next_arg(
        &mut self,
        i: usize,
        args: &[String],
        error: CommandError,
    ) -> Result<(), CommandError> {
        if i < args.len() - 1 && Self::is_flag(&args[i + 1]) {
            return Err(error);
        }
        Ok(())
    }

    fn add_reuse_message_config(
        &mut self,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError> {
        let options = ["-C".to_string(), "--reuse-message".to_string()].to_vec();
        Self::check_errors_flags(i, args, &options)?;
        self.check_next_arg(i, args, CommandError::ReuseMessageNoValue)?;
        self.reuse_message = Some(args[i + 1].clone());
        Ok(i + 2)
    }

    fn add_message_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options = ["-m".to_string()].to_vec();
        Self::check_errors_flags(i, args, &options)?;
        self.check_next_arg(i, args, CommandError::CommitMessageNoValue)?;
        let mut new_message: String = String::new();
        if let Some(message) = &self.message {
            //se usó -m al menos 1 vez
            new_message = format!("{}\n\n", message)
        }
        new_message += &args[i + 1];
        self.message = Some(new_message);
        Ok(i + 2)
    }

    fn add_dry_run_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options = ["--dry-run".to_string()].to_vec();
        Self::check_errors_flags(i, args, &options)?;
        self.dry_run = true;
        Ok(i + 1)
    }

    fn add_quiet_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options = ["-q".to_string(), "--quiet".to_string()].to_vec();
        Self::check_errors_flags(i, args, &options)?;
        self.quiet = true;
        Ok(i + 1)
    }

    fn add_all_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        let options = ["-a".to_string(), "--all".to_string()].to_vec();
        Self::check_errors_flags(i, args, &options)?;
        self.all = true;
        Ok(i + 1)
    }

    fn add_pathspec_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if Self::is_flag(&args[i]) {
            return Err(CommandError::InvalidArguments);
        }
        self.files.push(args[i].clone());
        Ok(i + 1)
    }

    /// Comprueba si el flag es invalido. En ese caso, devuelve error.
    fn check_errors_flags(
        i: usize,
        args: &[String],
        options: &[String],
    ) -> Result<(), CommandError> {
        if !options.contains(&args[i]) {
            return Err(CommandError::WrongFlag);
        }
        Ok(())
    }

    fn get_enter_message_text() -> String {
        let mensaje = "# Please enter the commit message for your changes. Lines starting\n# with '#' will be ignored, and an empty message aborts the commit.\n#\n";
        mensaje.to_string()
    }

    fn run_enter_message(&self, stdin: &mut dyn Read) -> Result<String, CommandError> {
        let stdout = Self::get_enter_message_text();
        let branch_path = get_current_branch()?;
        let branch_split: Vec<&str> = branch_path.split("/").collect();
        let branch_name = branch_split[branch_split.len() - 1];
        //let status_output =
        println!(
            "{}# On branch {}\n# Output de status\n#\n",
            stdout, branch_name
        );
        let mut message = String::new();
        let end = "q".to_string();
        loop {
            let mut buf = [0; 1];
            if stdin.read_exact(&mut buf).is_err() {
                return Err(CommandError::StdinError);
            };
            let input = String::from_utf8_lossy(&buf).to_string();
            if Self::check_end_message(&input, &end) {
                break;
            }
            message += &input;
        }
        message = Self::ignore_commented_lines(message);

        if message.is_empty() {
            return Err(CommandError::CommitMessageEmptyValue);
        }

        Ok(message.trim().to_string())
    }

    fn check_end_message(message: &str, end: &str) -> bool {
        let split_message: Vec<String> = message.lines().map(String::from).collect();
        if let Some(last) = split_message.to_owned().last() {
            if last.to_owned() == end.to_string() {
                return true;
            }
        }
        false
    }

    fn ignore_commented_lines(message: String) -> String {
        let split_message: Vec<&str> = message
            .lines()
            .filter(|line| !line.trim_start().starts_with("#"))
            .collect();
        split_message.join("\n")
    }

    fn empty_staging_area() {}

    fn run(
        &self,
        stdin: &mut dyn Read,
        output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        if self.message.is_some() && self.reuse_message.is_some() {
            return Err(CommandError::MessageAndReuseError);
        }
        logger.log("Retreiving message");

        let message = {
            if let Some(message) = self.message.clone() {
                message
            } else if self.reuse_message.is_some() {
                "".to_string()
            } else {
                self.run_enter_message(stdin)?
            }
        };
        logger.log("Opening stagin_area");

        let mut staging_area = StagingArea::open()?;

        staging_area = if !self.files.is_empty() {
            let mut new_staging_area = staging_area.empty()?;
            for path in self.files.iter() {
                add::run_for_file(path, &mut new_staging_area, logger)?;
            }
            new_staging_area
        } else if self.all {
            let new_staging_area = staging_area.empty()?;
            self.run_all_config(stdin, output, logger)?;
            new_staging_area
        } else {
            staging_area
        };

        logger.log("Writing work dir tree");

        let working_tree_hash = staging_area.write_tree(logger)?;
        logger.log("Work dir writen");

        if !staging_area.has_changes()? {
            logger.log("Nothing to commit");
            // show status output
            return Ok(());
        } else {
            self.run_commit(logger, message, working_tree_hash)?;
        }

        Ok(())
    }

    fn run_all_config(
        &self,
        stdin: &mut dyn Read,
        output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), CommandError> {
        let add_args = [".".to_string()].to_vec();
        Add::run_from("add", &add_args, stdin, output, logger)?;
        Ok(())
    }

    fn run_commit(
        &self,
        logger: &mut Logger,
        message: String,
        working_tree_hash: String,
    ) -> Result<(), CommandError> {
        let last_commit_hash = get_last_commit()?;

        let mut parents: Vec<String> = Vec::new();
        if let Some(padre) = last_commit_hash {
            parents.push(padre);
        }

        let commit: CommitObject;

        commit = if let Some(commit_hash) = &self.reuse_message {
            Self::get_reused_commit(commit_hash.to_string(), parents, working_tree_hash, logger)?
        } else {
            Self::create_new_commit(message, parents, working_tree_hash, logger)?
        };

        logger.log("Commit object created");

        let mut git_object: GitObject = Box::new(commit);
        let commit_hash = objects_database::write(logger, &mut git_object)?;
        logger.log(&format!("Commit object saved in database {}", commit_hash));
        if !self.dry_run {
            logger.log(&format!("Updating last commit to {}", commit_hash));
            update_last_commit(&commit_hash)?;
            logger.log("Last commit updated");
            // show commit status
        }

        // if !self.quiet {
        //     //self.get_commit_output(commit)
        // }

        Ok(())
    }

    fn create_new_commit(
        message: String,
        parents: Vec<String>,
        working_tree_hash: String,
        logger: &mut Logger,
    ) -> Result<CommitObject, CommandError> {
        let config = Config::open()?;

        let Some(author_email) = config.get("user.email") else {
            return Err(CommandError::UserConfigurationError);
        };
        let Some(author_name) = config.get("user.name") else {
            return Err(CommandError::UserConfigurationError);
        };

        let author = Author::new(author_name, author_email);
        let commiter = Author::new(author_name, author_email);

        let datetime: DateTime<Local> = Local::now();
        let timestamp = datetime.timestamp();
        let offset = datetime.offset().local_minus_utc() / 60;
        logger.log(&format!("offset: {}", offset));
        let commit = CommitObject::new(
            parents,
            message,
            author,
            commiter,
            timestamp,
            offset,
            working_tree_hash,
        )?;
        Ok(commit)
    }

    fn get_reused_commit(
        commit_hash: String,
        parents: Vec<String>,
        working_tree_hash: String,
        logger: &mut Logger,
    ) -> Result<CommitObject, CommandError> {
        let other_commit = objects_database::read_object(&commit_hash, logger)?;
        if let Some((message, author, committer, timestamp, offset)) =
            other_commit.get_info_commit()
        {
            let commit = CommitObject::new(
                parents,
                message,
                author,
                committer,
                timestamp,
                offset,
                working_tree_hash,
            )?;
            return Ok(commit);
        }
        Err(CommandError::CommitLookUp(commit_hash))
    }

    /// Obtiene la fecha y hora actuales.
    fn get_timestamp() {
        let timestamp: DateTime<Local> = Local::now();
        // formateo para que se vea como el de git.
        timestamp.format("%a %b %e %H:%M:%S %Y %z").to_string();
    }

    fn get_commit_output(&self, commit: Commit) {
        /*
        [<branch name> <commit hash[0:6]>] <message>
         # file changed, # insertions(+), # deletions(-)
        delete mode <modo> <file_name>
        created mode <modo> <file_name>

        info necesaria: current_branch (get_head_branch), hash, message, etc

        let output_string = ...;

         */

        //let _ = write!(output, "{}", output_string)
    }

    fn get_status_output(&self, output: &mut dyn Write) -> Result<(), CommandError> {
        /*
        si el staging area está vacía, se usa el output de status.
         */

        /* let mut status = Status::new_default();
        status.get_output(output)?; */
        Ok(())
    }
}

fn update_last_commit(commit_hash: &str) -> Result<(), CommandError> {
    let currect_branch = get_head_ref()?;
    let branch_path = format!(".git/{}", currect_branch);
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(branch_path)
        .map_err(|_| CommandError::FileOpenError(currect_branch.clone()))?;
    file.write_all(commit_hash.as_bytes()).map_err(|error| {
        CommandError::FileWriteError(format!(
            "Error al escribir en archivo {}: {}",
            currect_branch,
            error.to_string()
        ))
    })?;
    Ok(())
}

/// Opens file in .git/HEAD and returns the branch name
fn get_head_ref() -> Result<String, CommandError> {
    let Ok(mut head_file) = File::open(".git/HEAD") else {
        return Err(CommandError::FileOpenError(".git/HEAD".to_string()));
    };
    let mut head_content = String::new();
    head_file
        .read_to_string(&mut head_content)
        .map_err(|error| {
            CommandError::FileReadError(format!(
                "Error abriendo .git/HEAD: {:?}",
                error.to_string()
            ))
        })?;

    let Some((_, head_ref)) = head_content.split_once(" ") else {
        return Err(CommandError::FileReadError(
            "Error leyendo .git/HEAD".to_string(),
        ));
    };
    Ok(head_ref.trim().to_string())
}
