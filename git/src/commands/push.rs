use std::{
    collections::HashMap,
    io::{Read, Write},
};

use git_lib::{
    command_errors::CommandError,
    git_repository::{push_branch_hashes, GitRepository},
    objects_database::get_last_commit_hash_branch,
};

use super::command::{Command, ConfigAdderFunction};

/// Commando Clone
pub struct Push {
    all: bool,
    remote: String,
    branch: String,
}

impl Command for Push {
    fn run_from(
        name: &str,
        args: &[String],
        stdin: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), CommandError> {
        if name != "push" {
            return Err(CommandError::Name);
        }
        let push = Push::new(args, output)?;
        push.run(stdin, output)?;
        Ok(())
    }

    fn config_adders(&self) -> ConfigAdderFunction<Push> {
        vec![Push::add_all_config]
    }
}

impl Push {
    fn new(args: &[String], output: &mut dyn Write) -> Result<Push, CommandError> {
        if args.len() > 2 {
            return Err(CommandError::InvalidArguments);
        }
        let mut push = Push::new_default(output)?;
        push.config(args)?;
        Ok(push)
    }

    fn new_default(output: &mut dyn Write) -> Result<Push, CommandError> {
        let mut repo = GitRepository::open("", output).unwrap();
        let current_branch = repo.get_current_branch_name()?;

        Ok(Push {
            all: false,
            remote: "origin".to_string(),
            branch: current_branch,
        })
    }

    fn add_all_config(push: &mut Push, i: usize, args: &[String]) -> Result<usize, CommandError> {
        if args[i] == "--all" {
            if args.len() > 1 {
                return Err(CommandError::InvalidArguments);
            }
            push.all = true;
            Ok(i + 1)
        } else {
            Ok(i)
        }
    }

    fn run(&self, _stdin: &mut dyn Read, output: &mut dyn Write) -> Result<(), CommandError> {
        let mut repo = GitRepository::open("", output)?;
        let mut local_branches: Vec<(String, String)> = Vec::new(); // (branch, hash)

        if self.all {
            push_branch_hashes(&mut local_branches)?;
        } else {
            let hash_commit = get_last_commit_hash_branch(&self.branch)?;
            local_branches.push((self.branch.to_owned(), hash_commit));
        }

        let hash_analysis = repo.push_analysis(local_branches)?;
        Ok(())
    }
}
