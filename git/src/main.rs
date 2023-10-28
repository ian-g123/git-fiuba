use git::commands::{
    add_components::add::Add, cat_file_components::cat_file::CatFile, command::Command,
    commit_components::commit::Commit, hash_object_components::hash_object::HashObject,
    init_components::init::Init, status_components::status::Status,
};
use git_lib::{command_errors::CommandError, logger::Logger};
use std::{env, io};

fn main() {
    let args: Vec<String> = env::args().collect();
    let (command_name, command_args) = parse_args(&args);

    let mut logger = Logger::new(".git/logs.txt").unwrap();

    if let Err(error) = run(command_name, command_args, &mut logger) {
        logger.log(&format!("Error: {}", error));
        eprintln!("{error}")
    }
}

fn parse_args(args: &[String]) -> (&str, &[String]) {
    let command = &args[1];
    let command_args = args.split_at(2).1;
    (command, command_args)
}

fn run(
    command_name: &str,
    command_args: &[String],
    logger: &mut Logger,
) -> Result<(), CommandError> {
    let commands = [
        HashObject::run_from,
        Init::run_from,
        Add::run_from,
        CatFile::run_from,
        Commit::run_from,
        Status::run_from,
    ];

    for command in &commands {
        match command(
            command_name,
            command_args,
            &mut io::stdin(),
            &mut io::stdout(),
            logger,
        ) {
            Ok(()) => return Ok(()),
            Err(CommandError::Name) => {}
            Err(error) => return Err(error),
        }
    }
    Err(CommandError::Name)
}

// fn parse_args(args: &[String]) -> Result<(&str, &[String]), ErrorFlags> {
//     if args.len() == 1 {
//         return Err(ErrorFlags::ArgsNumber);
//     }
//     let command = &args[1];
//     let command_args = args.split_at(2).1;

//     Ok((command, command_args))
// }

// fn ejecutar(command_name: &str, command_args: &[String]) -> Result<(), ErrorFlags> {
//     let commands = [HashObject::run];

//     for command in &commands {
//         match command(command_name, command_args) {
//             Ok(()) => return Ok(()),
//             Err(ErrorFlags::CommandName) => {}
//             Err(error) => return Err(error),
//         }
//     }
//     Err(ErrorFlags::ArgsNumber)
// }
