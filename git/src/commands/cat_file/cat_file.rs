use crate::commands::{command::Command, error_flags::ErrorFlags};
use std::{io::{Read, Write}, path::Path};

pub struct CatFile {
    hash: String,
    pretty: bool,
    type_object: bool,
    size: bool,
}

impl Command for CatFile {
    fn run_from(
        name: &str,
        args: &[String],
        input: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), ErrorFlags> {
        if name != "cat-file" {
            return Err(ErrorFlags::CommandName);
        }

        let mut cat_file = CatFile::new(args)?;
        cat_file.run(input, output)
    }

    fn is_flag(arg: &str) -> bool {
        if arg.starts_with('-') && arg.len() == 2 {
            let flag = match arg.chars().nth(1).ok_or(ErrorFlags::WrongFlag) {
                Ok(flag) => flag,
                Err(_) => return false,
            };

            if ['p', 't', 's'].contains(&flag) {
                return true;
            }
        }
        false
    }
}

impl CatFile {
    fn new(args: &[String]) -> Result<CatFile, ErrorFlags> {
        let hash = valid_arguments(args)?;

        let cat_file = CatFile {
            hash,
            pretty: args.contains(&String::from("-p")),
            type_object: args.contains(&String::from("-t")),
            size: args.contains(&String::from("-s")),
        };

        Ok(cat_file)
    }

    fn run(&self, _input: &mut dyn Read, output: &mut dyn Write) -> Result<(), ErrorFlags> {
        Ok(())
    }
}

fn valid_arguments(args: &[String]) -> Result<String, ErrorFlags> {
    if args.is_empty() {
        return Err(ErrorFlags::InvalidArguments);
    }
    for arg in args {
        if !CatFile::is_flag(arg) {
            return Err(ErrorFlags::WrongFlag);
        }
    }
    let hash = match args.last() {
        Some(hash) => hash,
        None => return Err(ErrorFlags::InvalidArguments),
    };
    let hash = format!(".git/objects/{}/{}", &hash[..2], &hash[2..]);
    if !Path::new(&hash).exists() {
        return Err(ErrorFlags::InvalidArguments);
    }
    Ok(hash)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    // Se usará de test el archivo new.txt que se encuentra en test/data/
    // que indica que el hash del archivo es fa49b077972391ad58037050f2a75f74e3671e92 y su contenido es
    // new file
    use super::*;

    #[test]
    fn test_invalid_name() {
        let mut output_string: Vec<u8> = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = vec![];
        let result = CatFile::run_from("cat-", &args, &mut stdin_mock, &mut stdout_mock);
        assert!(matches!(result, Err(ErrorFlags::CommandName)))
    }

    #[test]
    fn test_invalid_flag() {
        let mut output_string: Vec<u8> = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = vec!["-".to_string()];
        let result = CatFile::run_from("cat-file", &args, &mut stdin_mock, &mut stdout_mock);
        assert!(matches!(result, Err(ErrorFlags::WrongFlag)))
    }

    #[test]
    fn test_invalid_flag2() {
        let mut output_string: Vec<u8> = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = vec!["-h".to_string()];
        let result = CatFile::run_from("cat-file", &args, &mut stdin_mock, &mut stdout_mock);
        assert!(matches!(result, Err(ErrorFlags::WrongFlag)))
    }

//     #[test]
//     fn test_incorrect_hash_print_an_error() {
//         let mut output = Vec::new();
//         let mut input = Vec::new();

//         let args = vec![];
//         let result = CatFile::run_from("cat-file", &args, &mut input, &mut output);
//         assert!(result.is_err());
//         assert_eq!(
//             String::from_utf8(output).unwrap(),
//             "fatal: Not a valid object name incorrect_hash\n"
//         );
//     }
}
