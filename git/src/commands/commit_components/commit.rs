use std::{io::Read, io::Write};

use crate::{commands::{command::{Command, ConfigAdderFunction}, command_errors::CommandError, init_components::init::Init}, logger::Logger};

pub struct Commit {
    all: bool,
    reuse_message: Option<String>,
    dry_run: bool,
    message: Option<String>,
    quiet: bool,
    files: Vec<String>
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

        let instance = Self::new_from(args)?;

        logger.log(&format!("commit {:?}", args));
        instance.run(output)?;
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
        Commit { all: false, reuse_message: None, dry_run: false, message: None, quiet: false, files: Vec::new()}
    }

    fn check_next_arg(&mut self, i: usize,
        args: &[String], error: CommandError)-> Result<(), CommandError>{
            if i < args.len() - 1 && Self::is_flag(&args[i+1]){
                return Err(error);
            }
            Ok(())
    }

    fn add_reuse_message_config(&mut self,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError>{
        let options = ["-C".to_string(), "--reuse-message".to_string()].to_vec();
        Self::check_errors_flags(i, args, &options)?;
        self.check_next_arg(i, args, CommandError::ReuseMessageNoValue)?;
        self.reuse_message = Some(args[i+1].clone());
        Ok(i+2)
    }

    fn add_message_config(&mut self,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError>{
        let options = ["-m".to_string()].to_vec();
        Self::check_errors_flags(i, args, &options)?;
        self.check_next_arg(i, args, CommandError::CommitMessageNoValue)?;
        self.message = Some(args[i+1].clone());
        Ok(i+2)
    }

    fn add_dry_run_config(&mut self,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError>{
        let options = ["--dry-run".to_string()].to_vec();
        Self::check_errors_flags(i, args, &options)?;
        self.dry_run = true;
        Ok(i+1)
    }


    fn add_quiet_config(&mut self,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError>{
        let options = ["-q".to_string(), "--quiet".to_string()].to_vec();
        Self::check_errors_flags(i, args, &options)?;
        self.quiet = true;
        Ok(i+1)
    }

    fn add_all_config(&mut self,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError>{
        let options = ["-a".to_string(), "--all".to_string()].to_vec();
        Self::check_errors_flags(i, args, &options)?;
        self.all = true;
        Ok(i+1)
    }

    fn add_pathspec_config(&mut self,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError>{
        if Self::is_flag(&args[i]){
            return Err(CommandError::InvalidArguments);
        }
        self.files.push(args[i].clone());
        Ok(i+1)
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

    fn run(&self, output: &mut dyn Write)->Result<(), CommandError>{

        if self.message.is_some() && self.reuse_message.is_some(){
            return Err(CommandError::MessageAndReuseError)
        }

        /* 
        if staging_area.is_empty() && self.files.is_empty(){
            self.set_nothing_to_commit_output(output)?;
        }
        */

        /* paths, C, m, dry-run, reuse-message, q, all */



        Ok(())
    }

    //error: pathspec '<path>>' did not match any file(s) known to git

    fn add_files_to_index(){

    }

    fn set_nothing_to_commit_output(&self, output: &mut dyn Write)->Result<(), CommandError>{

        /* 
        si el staging area está vacía, se usa el output de status.
         */

        /* let mut status = Status::new_default();
        status.get_output(output)?; */
       Ok(())
    }
}