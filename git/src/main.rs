use git::commands::{
    add::Add, branch::Branch, cat_file::CatFile, check_ignore::CheckIgnore, checkout::Checkout,
    clone::Clone, command::Command, commit::Commit, fetch::Fetch, hash_object::HashObject,
    init::Init, log::Log, ls_files::LsFiles, ls_tree::LsTree, merge::Merge, push::Push,
    rebase::Rebase, show_ref::ShowRef, status::Status, tag::Tag,
};
use git_lib::command_errors::CommandError;
use std::{env, io};

fn main() {
    let args: Vec<String> = env::args().collect();
    let (command_name, command_args) = parse_args(&args);

    if let Err(error) = run(command_name, command_args) {
        eprintln!("{error}")
    }
}

fn parse_args(args: &[String]) -> (&str, &[String]) {
    if args.len() < 2 {
        panic!("Not enough arguments");
    }

    let command = &args[1];
    let command_args = args.split_at(2).1;
    (command, command_args)
}

fn run(command_name: &str, command_args: &[String]) -> Result<(), CommandError> {
    let commands = [
        HashObject::run_from,
        Init::run_from,
        Add::run_from,
        CatFile::run_from,
        Commit::run_from,
        Status::run_from,
        Clone::run_from,
        Fetch::run_from,
        Merge::run_from,
        Push::run_from,
        Log::run_from,
        Branch::run_from,
        Checkout::run_from,
        LsFiles::run_from,
        Tag::run_from,
        Rebase::run_from,
        ShowRef::run_from,
        LsTree::run_from,
        CheckIgnore::run_from,
    ];

    for command in &commands {
        match command(
            command_name,
            command_args,
            &mut io::stdin(),
            &mut io::stdout(),
        ) {
            Ok(()) => return Ok(()),
            Err(CommandError::Name) => {}
            Err(error) => return Err(error),
        }
    }
    Err(CommandError::Name)
}
