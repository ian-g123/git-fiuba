use std::{io::Read, io::Write};

use crate::{commands::{command::{Command, ConfigAdderFunction}, command_errors::CommandError}, logger::Logger};

pub struct Commit {
    all: bool,
    reuse_message: Option<String>,
    dry_run: bool,
    message: Option<String>,
    quiet: bool
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
        //instance.run()?;
        Ok(())
    }

    fn config_adders(&self) -> ConfigAdderFunction<Self> {
        vec![
            Self::add_all_config,
            Self::add_dry_run_config,
            Self::add_message_config,
            Self::add_quiet_config,
            Self::add_reuse_message_config
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
        Commit { all: false, reuse_message: None, dry_run: false, message: None, quiet: false}
    }

    fn check_next_arg(&mut self, i: usize,
        args: &[String])-> Result<(), CommandError>{
            if i < args.len() - 1 && Self::is_flag(&args[i+1]){
                return Err(CommandError::InvalidArguments);
            }
            Ok(())
    }

    fn add_reuse_message_config(&mut self,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError>{
        let options = ["-C".to_string(), "--reuse-message".to_string()].to_vec();
        self.check_errors_flags(i, args, &options)?;
        self.check_next_arg(i, args)?;
        self.reuse_message = Some(args[i+1].clone());
        Ok(i+2)
    }

    fn add_message_config(&mut self,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError>{
        let options = ["-m".to_string()].to_vec();
        self.check_errors_flags(i, args, &options)?;
        self.check_next_arg(i, args)?;
        self.message = Some(args[i+1].clone());
        Ok(i+2)
    }

    fn add_dry_run_config(&mut self,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError>{
        let options = ["--dry-run".to_string()].to_vec();
        self.check_errors_flags(i, args, &options)?;
        self.dry_run = true;
        Ok(i+1)
    }


    fn add_quiet_config(&mut self,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError>{
        let options = ["-q".to_string(), "--quiet".to_string()].to_vec();
        self.check_errors_flags(i, args, &options)?;
        self.quiet = true;
        Ok(i+1)
    }

    fn add_all_config(&mut self,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError>{
        let options = ["-a".to_string(), "--all".to_string()].to_vec();
        self.check_errors_flags(i, args, &options)?;
        self.all = true;
        Ok(i+1)
    }

    /// Comprueba si el flag es invalido. En ese caso, devuelve error.
    fn check_errors_flags(
        &self,
        i: usize,
        args: &[String],
        options: &[String],
    ) -> Result<(), CommandError> {
        if !options.contains(&args[i]) {
            return Err(CommandError::WrongFlag);
        }
        Ok(())
    }
}