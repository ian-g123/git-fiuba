use super::command::{Command, ConfigAdderFunction};
use git_lib::{command_errors::CommandError, git_repository::GitRepository};
use std::io::{Read, Write};

/// Commando Rebase
pub struct Rebase {
    continue_rebase: bool,
    abort_rebase: bool,
    topic_branch: String,
    main_branch: String,
}

impl Command for Rebase {
    fn run_from(
        name: &str,
        args: &[String],
        _stdin: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        if name != "rebase" {
            return Err(CommandError::Name);
        }
        let rebase = Rebase::new(args, output)?;
        rebase.run(output)?;
        Ok(())
    }

    fn config_adders(&self) -> ConfigAdderFunction<Rebase> {
        vec![Rebase::continue_config, Rebase::abort_config]
    }
}

impl Rebase {
    fn new(args: &[String], output: &mut dyn Write) -> Result<Rebase, CommandError> {
        if args.len() < 1 {
            return Err(CommandError::RebaseError("There is no tracking information for the current branch.\nPlease specify which branch you want to rebase against.\nSee git-rebase(1) for details.\ngit rebase '<branch>'\nIf you wish to set tracking information for this branch you can do so with:\ngit branch --set-upstream-to=<remote>/<branch> rama".to_string()));
        }
        let mut rebase = Rebase::new_default()?;

        rebase.config(args)?;

        if args.len() > 1 {
            rebase.topic_branch = args[0].clone();
            rebase.main_branch = args[1].clone();
        }

        if args.len() == 1 && !rebase.continue_rebase && !rebase.abort_rebase {
            rebase.topic_branch = args[0].clone();
            let mut repo = GitRepository::open("", output)?;
            rebase.main_branch = repo.get_current_branch_name()?;
        }

        Ok(rebase)
    }

    pub fn new_default() -> Result<Rebase, CommandError> {
        Ok(Rebase {
            continue_rebase: false,
            abort_rebase: false,
            topic_branch: "".to_string(),
            main_branch: "".to_string(),
        })
    }

    fn continue_config(
        rebase: &mut Rebase,
        i: usize,
        args: &[String],
    ) -> Result<usize, CommandError> {
        if args[i] == "--continue" {
            rebase.continue_rebase = true;
        }
        Ok(i + 1)
    }

    fn abort_config(rebase: &mut Rebase, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if args[i] == "--abort" {
            rebase.abort_rebase = true;
        }
        Ok(i + 1)
    }

    pub fn run(&self, output: &mut dyn Write) -> Result<(), CommandError> {
        let mut repo = GitRepository::open("", output)?;
        if !self.abort_rebase & !self.continue_rebase {
            //verificar que no haya un rebase en proceso
            match repo.initialize_rebase(self.topic_branch.clone(), self.main_branch.clone()) {
                Err(CommandError::RebaseMergeConflictsError) => {
                    //usuario resolver conflictos
                }
                Err(error) => return Err(error),
                Ok(_) => {
                    let message = "Successfully rebased and updated refs/heads/".to_string()
                        + &self.main_branch.clone();
                    _ = output.write(message.as_bytes());
                    return Ok(());
                }
            };
        }
        if self.abort_rebase {
            //abort!!
        }
        // println!("continue_rebase: {}", self.continue_rebase);
        if self.continue_rebase {
            match repo.merge_continue_rebase() {
                Err(CommandError::RebaseMergeConflictsError) => {
                    let error_message = repo.print_error_merge_conflicts()?;
                    return Err(CommandError::RebaseError(error_message));
                }
                Err(error) => return Err(error),
                Ok(_) => {}
            };
        }
        Ok(())
    }
}
