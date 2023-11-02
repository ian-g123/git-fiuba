use std::io::{Read, Write};

use git_lib::{command_errors::CommandError, git_repository::GitRepository};

use super::command::{Command, ConfigAdderFunction};

/// Commando Merge
pub struct Merge {
    comits: Vec<String>,
    continue_: bool,
    abort: bool,
    quit: bool,
}

impl Command for Merge {
    fn run_from(
        name: &str,
        args: &[String],
        stdin: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        if name != "merge" {
            return Err(CommandError::Name);
        }
        let instance = Self::new(args)?;

        instance.run(stdin, output)?;
        Ok(())
    }

    fn config_adders(&self) -> ConfigAdderFunction<Merge> {
        vec![
            Merge::add_continue_config,
            Merge::add_abort_config,
            Merge::add_quit_config,
            Merge::add_comit_config,
        ]
    }
}

impl Merge {
    fn new(args: &[String]) -> Result<Merge, CommandError> {
        let mut merge = Merge::new_default();
        merge.config(args)?;

        Ok(merge)
    }

    fn new_default() -> Merge {
        Merge {
            comits: Vec::new(),
            continue_: false,
            abort: false,
            quit: false,
        }
    }

    /// Configura el flag --continue.
    fn add_continue_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if args[i] != "--continue" {
            return Err(CommandError::WrongFlag);
        }
        if self.abort || self.quit {
            return Err(CommandError::MergeOneOperation);
        }
        self.continue_ = true;
        Ok(i + 1)
    }

    /// Configura el flag --abort.
    fn add_abort_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if args[i] != "--abort" {
            return Err(CommandError::WrongFlag);
        }
        if self.continue_ || self.quit {
            return Err(CommandError::MergeOneOperation);
        }
        todo!("merge --abort no está hecho");
        self.abort = true;
        Ok(i + 1)
    }

    /// Configura el flag --quit.
    fn add_quit_config(&mut self, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if args[i] != "--quit" {
            return Err(CommandError::WrongFlag);
        }
        if self.abort || self.continue_ {
            return Err(CommandError::MergeOneOperation);
        }
        todo!("merge --quit no está hecho");
        self.quit = true;
        Ok(i + 1)
    }

    fn add_comit_config(
        merge: &mut Merge,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError> {
        merge.comits.push(args[i].clone());
        Ok(i + 1)
    }

    fn run(&self, _stdin: &mut dyn Read, output: &mut dyn Write) -> Result<(), CommandError> {
        let mut repo = GitRepository::open("", output)?;
        if self.continue_ {
            return repo.merge_continue();
        }
        if self.abort {
            // return repo.merge_abort();
        }
        if self.quit {
            // return repo.merge_quit();
        }

        repo.merge(&self.comits)?;
        Ok(())
    }
}
