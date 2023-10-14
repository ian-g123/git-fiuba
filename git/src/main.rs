use git::{
    commands::{
        add_components::add::Add, command::Command, command_errors::CommandError,
        hash_object_components::hash_object::HashObject, init_components::init::Init,
    },
    logger::Logger,
};
use std::{env, io};

fn main() {
    let args: Vec<String> = env::args().collect();
    let Ok((command_name, command_args)) = parse_args(&args) else {
        eprint!("Error");
        return;
    };

    let Ok(mut logger) = Logger::new(".git/logs") else {
        return;
    };

    if let Err(error) = run(command_name, command_args, &mut logger) {
        eprintln!("{error}")
    }
}

fn parse_args(args: &[String]) -> Result<(&str, &[String]), CommandError> {
    let command = &args[1];
    let command_args = args.split_at(2).1;

    Ok((command, command_args))
}

fn run(
    command_name: &str,
    command_args: &[String],
    logger: &mut Logger,
) -> Result<(), CommandError> {
    let commands = [HashObject::run_from, Init::run_from, Add::run_from];

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
