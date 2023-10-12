use crate::{
    commands::{command::Command, error_flags::ErrorFlags},
    logger::Logger,
};
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

extern crate libflate;

pub struct CatFile {
    path: String,
    exists: bool,
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
        logger: &mut Logger,
    ) -> Result<(), ErrorFlags> {
        if name != "cat-file" {
            return Err(ErrorFlags::CommandName);
        }

        let cat_file = CatFile::new(args)?;
        cat_file.run(output)
    }

    fn is_flag(arg: &str) -> bool {
        if arg.starts_with('-') && arg.len() == 2 {
            let flag = match arg.chars().nth(1).ok_or(ErrorFlags::WrongFlag) {
                Ok(flag) => flag,
                Err(_) => return false,
            };

            if ['p', 't', 's', 'e'].contains(&flag) {
                return true;
            }
        }
        false
    }

    fn config_adders(&self) -> Vec<fn(&mut Self, usize, &[String]) -> Result<usize, ErrorFlags>> {
        todo!()
    }
}

impl CatFile {
    fn new(args: &[String]) -> Result<CatFile, ErrorFlags> {
        let path = valid_arguments(args)?;

        let exists = args.contains(&String::from("-e"));
        let pretty = args.contains(&String::from("-p"));
        let type_object = args.contains(&String::from("-t"));
        let size = args.contains(&String::from("-s"));

        if exists && (pretty || type_object || size) {
            return Err(ErrorFlags::OptionCombinationError);
        }

        let cat_file = CatFile {
            exists,
            path,
            pretty,
            type_object,
            size,
        };

        Ok(cat_file)
    }

    fn run(&self, output: &mut dyn Write) -> Result<(), ErrorFlags> {
        println!("{}", &self.path);
        let path = Path::new(&self.path);

        if !path.exists() {
            return Err(ErrorFlags::FileNotFound);
        }

        let data = decompress_data(path)?;

        let (header, content) = match data.split_once('\0') {
            Some((header, content)) => (header, content),
            None => return Err(ErrorFlags::ObjectTypeError),
        };

        let (object_type, size) = match header.split_once(' ') {
            Some((object_type, size)) => (object_type, size),
            None => return Err(ErrorFlags::ObjectTypeError),
        };

        self.show_in_output(output, object_type, size, content);
        Ok(())
    }

    fn show_in_output(&self, output: &mut dyn Write, object_type: &str, size: &str, content: &str) {
        if self.exists {
            let _ = writeln!(output, "exists");
            return;
        }

        if self.type_object {
            let _ = writeln!(output, "{}", object_type);
        }

        if self.size {
            let _ = writeln!(output, "{}", size);
        }

        if self.pretty {
            let _ = writeln!(output, "{}", content);
        }
    }
}

/// Descomprime el archivo y devuelve su contenido
fn decompress_data(path: &Path) -> Result<String, ErrorFlags> {
    let compressed_file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return Err(ErrorFlags::FileReadError),
    };

    let mut decoder = libflate::deflate::Decoder::new(&compressed_file);
    let mut data = String::new();

    match decoder.read_to_string(&mut data) {
        Ok(_) => (),
        Err(_) => return Err(ErrorFlags::DecompressError),
    };
    Ok(data)
}

/// Verifica que los argumentos sean válidos y devuelve el hash proporcionado por el usuario
fn valid_arguments(args: &[String]) -> Result<String, ErrorFlags> {
    if args.len() < 2 {
        return Err(ErrorFlags::NotEnoughArguments);
    }
    for arg in &args[..args.len() - 1] {
        if !CatFile::is_flag(arg) {
            return Err(ErrorFlags::WrongFlag);
        }
    }
    let hash = match args.last() {
        Some(hash) => hash,
        None => return Err(ErrorFlags::InvalidArguments),
    };
    // Ok(format!(".git/objects/{}/{}", &hash[..2], &hash[2..])) FORMA CORRECTA (USAR CUANDO ESTÉ IMPLEMENTADO GIT INIT) CORREGIR TEST AL MODIFICAR
    Ok(format!("test/data/objects/{}/{}", &hash[..2], &hash[2..]))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    // Se usará de test el archivo new.txt que se encuentra en test/data/
    // que indica que el hash del archivo es fa49b077972391ad58037050f2a75f74e3671e92 y su contenido es
    // new file

    // MODIFICAR TODO AL IMPLEMENTAR GIT INIT
    use super::*;

    #[test]
    fn test_invalid_name() {
        let mut logger = Logger::new(".git/logs").unwrap();
        let mut output_string: Vec<u8> = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = vec![];
        let result = CatFile::run_from(
            "cat-",
            &args,
            &mut stdin_mock,
            &mut stdout_mock,
            &mut logger,
        );
        assert!(matches!(result, Err(ErrorFlags::CommandName)))
    }

    #[test]
    fn test_invalid_flag_name() {
        let mut logger = Logger::new(".git/logs").unwrap();
        let mut output_string: Vec<u8> = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = vec!["-".to_string()];
        let result = CatFile::run_from(
            "cat-file",
            &args,
            &mut stdin_mock,
            &mut stdout_mock,
            &mut logger,
        );
        assert!(matches!(result, Err(ErrorFlags::WrongFlag)))
    }

    #[test]
    fn test_invalid_flag_name2() {
        let mut logger = Logger::new(".git/logs").unwrap();
        let mut output_string: Vec<u8> = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = vec!["-h".to_string()];
        let result = CatFile::run_from(
            "cat-file",
            &args,
            &mut stdin_mock,
            &mut stdout_mock,
            &mut logger,
        );
        assert!(matches!(result, Err(ErrorFlags::WrongFlag)))
    }

    #[test]
    fn test_not_enough_arguments() {
        let mut logger = Logger::new(".git/logs").unwrap();
        let mut output_string: Vec<u8> = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = vec!["-p".to_string()];
        let result = CatFile::run_from(
            "cat-file",
            &args,
            &mut stdin_mock,
            &mut stdout_mock,
            &mut logger,
        );
        assert!(matches!(result, Err(ErrorFlags::NotEnoughArguments)))
    }

    #[test]
    fn test_incorrect_hash_return_error() {
        let mut logger = Logger::new(".git/logs").unwrap();
        let mut output_string: Vec<u8> = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args = vec![
            "-p".to_string(),
            "fa49b077972391ad58037050f2a75f74e3671e92".to_string(),
        ];
        let result = CatFile::run_from(
            "cat-file",
            &args,
            &mut stdin_mock,
            &mut stdout_mock,
            &mut logger,
        );
        assert!(matches!(result, Err(ErrorFlags::InvalidArguments)))
    }
}
