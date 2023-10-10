use std::env;
use git::commands::command::Command;
use git::commands::hash_object_components::hash_object::HashObject;
use git::error_args::ErrorFlags;

fn main() {
    // let args: Vec<String> = env::args().collect();
    // let Ok((command_name, command_args)) = parse_args(&args) else {
    //     eprint!("Error");
    //     return;
    // };

    let command_name = "hash-object";  // se usa para debuguear !!!NO BORRAR
    let command_args = &["-t".to_string(), "blob".to_string(), "--stdin".to_string(), "-w".to_string(), "--path".to_string(), "file.txt".to_string()];

    _ = ejecutar(command_name, command_args);
}

fn parse_args(args: &[String]) -> Result<(&str, &[String]), ErrorFlags> {
    if args.len() == 1 {
        return Err(ErrorFlags::ArgsNumber);
    }
    let command = &args[1];
    let command_args = args.split_at(2).1;

    Ok((command, command_args))
}

fn ejecutar(command_name: &str, command_args: &[String]) -> Result<(), ErrorFlags> {
    let commands = [HashObject::run];

    for command in &commands {
        match command(command_name, command_args) {
            Ok(()) => return Ok(()),
            Err(ErrorFlags::CommandName) => {}
            Err(error) => return Err(error),
        }
    }
    Err(ErrorFlags::ArgsNumber)
}
