use std::{
    fs::{self, File, OpenOptions},
    io::Read,
    io::Write,
};

use chrono::{DateTime, Local};

use crate::{
    commands::{
        branch_manager::get_last_commit,
        command::{Command, ConfigAdderFunction},
        command_errors::CommandError,
        config::Config,
        init_components::init::Init,
        objects::{author::Author, commit_object::CommitObject, tree::Tree},
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

        let instance = Self::new_from(args)?;

        instance.run(output, logger)?;
        logger.log(&format!("commit {:?}", args));
        Ok(())
    }

    fn config_adders(&self) -> ConfigAdderFunction<Self> {
        vec![
            Self::add_all_config,
            Self::add_dry_run_config,
            Self::add_message_config,
            Self::add_quiet_config,
            Self::add_reuse_message_config,
            Self::add_pathspec_config,
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
        self.message = Some(args[i + 1].clone());
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
        let mensaje = "# Please enter the commit message for your changes. Lines starting\n# with '#' will be ignored, and an empty message aborts the commit.\n\n";
        //mensaje  = format!("{}{}", mensaje, get_status_output);
        mensaje.to_string()
    }

    fn run_enter_message(&self) {
        let stdout = Self::get_enter_message_text();
        /*
        1) read from stdin
        2) Ignore # lines
        3) message
         */
    }

    fn run(&self, output: &mut dyn Write, logger: &mut Logger) -> Result<(), CommandError> {
        if self.message.is_some() && self.reuse_message.is_some() {
            return Err(CommandError::MessageAndReuseError);
        }

        let message = {
            if let Some(message) = self.message.clone() {
                message
            } else if let Some(reuse_message) = self.reuse_message.clone() {
                Self::get_commit_reuse()
            } else {
                return Err(CommandError::CommitMessageEmptyValue);
            }
        };

        let stagin_area = StagingArea::open()?;

        let working_tree_hash = stagin_area.write_tree(logger)?;

        let config = Config::open()?;

        let Some(author_email) = config.get("user.email") else {
            return Err(CommandError::UserConfigurationError);
        };
        let Some(author_name) = config.get("user.name") else {
            return Err(CommandError::UserConfigurationError);
        };

        let author = Author::new(author_name, author_email);
        let commiter = Author::new(author_name, author_email);

        let last_commit_hash = get_last_commit()?;

        let mut padres: Vec<String> = Vec::new();
        if let Some(padre) = last_commit_hash {
            padres.push(padre);
        }

        let datetime: DateTime<Local> = Local::now();
        let commit = CommitObject::new(
            padres,
            message,
            author,
            commiter,
            datetime,
            working_tree_hash,
        )?;

        let commit_hash = objects_database::write(Box::new(commit))?;
        if !self.dry_run {
            update_last_commit(&commit_hash)?;
        }
        //Commit
        /*
        if staging_area.is_empty() && self.files.is_empty(){
            self.set_nothing_to_commit_output(output)?;
        }
        */

        // for path in self.files.iter() {
        //     /*
        //     1) vaciar StagingArea o crear una nueva
        //     2) Agregar al stagingArea. Manejar error si no está en la base de datos o no existe
        //      */
        // }

        if self.message.is_none() {
            self.run_enter_message();
        }

        if let Some(commit_hash) = self.reuse_message.clone() {}

        //Crear Commit Object con la info necesaria --> Commit::new()

        // if self.dry_run {
        //     self.get_status_output(output)?;
        // } else {
        //     //self.add_commit(commit)
        // }

        // if !self.quiet {
        //     //self.get_commit_output(commit)
        // }

        Ok(())
    }

    /// Obtiene la fecha y hora actuales.
    fn get_timestamp() {
        let timestamp: DateTime<Local> = Local::now();
        // formateo para que se vea como el de git.
        timestamp.format("%a %b %e %H:%M:%S %Y %z").to_string();
    }

    fn get_commit_reuse() -> String {
        "".to_string()
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

    //error: pathspec '<path>>' did not match any file(s) known to git

    fn add_files_to_index() {}

    fn add_commit(&mut self, commit: Commit) {
        /*
        1) commit.get_hash() -> guardar en la base de datos
        2) Actualizar current branch en .git para que apunte al nuevo commit -->
            a- .git/HEAD --> ref: refs/heads/<current_branch> --> get_head_branch
            b- refs/heads/<current_branch> --> write commit hash
         */
    }

    /// Obtiene la rama actual. Si no se puede leer de ".git/HEAD", se devuelve error.

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
        .map_err(|_| CommandError::FileOpenError(currect_branch))?;
    file.write_all(commit_hash.as_bytes());
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
    Ok(head_ref.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /* #[test]
    fn get_current_branch(){
        assert!(Commit::get_head_branch().is_ok())
    }  */
}
