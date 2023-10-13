use std::fs::{File, self};
use std::io::{Read, Write, self};
use std::{str, env};


use crate::commands::command::Command;
use crate::commands::error_flags::ErrorFlags;
use crate::logger::Logger;

/// Commando init
pub struct Init {
    branch_main: String,
    working_directory: bool,
    files: Vec<String>,
}

impl Command for Init {
    fn run_from(
        name: &str,
        args: &[String],
        stdin: &mut dyn Read,
        output: &mut dyn Write,
        logger: &mut Logger,
    ) -> Result<(), ErrorFlags> {
        if name != "init" {
            return Err(ErrorFlags::CommandName);
        }

        let mut instance = Self::new(args)?;

        if instance.files.len() == 0 {
            let current_dir = env::current_dir().map_err(|_| ErrorFlags::InvalidArguments)?;
            let current_dir_display = current_dir.display();
            instance.files.push(current_dir_display.to_string());
        }

        instance.run(output)?;
        Ok(())
    }

    fn config_adders(&self) -> Vec<fn(&mut Self, usize, &[String]) -> Result<usize, ErrorFlags>> {
        vec![
            Self::add_bare_config,
            Self::add_branch_config,
            Self::add_file_config,
        ]
    }
}

impl Init {
    fn new(args: &[String]) -> Result<Self, ErrorFlags> {
        let mut init = Self::new_default();
        init.config(args)?;
        Ok(init)
    }

    fn new_default() -> Self {
        Self {
            branch_main: "main".to_string(),
            working_directory : true,
            files: Vec::<String>::new(),
        }
    }

    fn add_bare_config(
        init: &mut Init,
        i: usize,
        args: &[String],
    ) -> Result<usize, ErrorFlags> {
        if args[i] != "--bare" {
            return Err(ErrorFlags::WrongFlag);
        }
        init.working_directory = false;
        Ok(i + 1)
    }

    fn add_branch_config(
        init: &mut Init,
        i: usize,
        args: &[String],
    ) -> Result<usize, ErrorFlags> {
        if args[i] != "-b" {
            return Err(ErrorFlags::WrongFlag);
        }
        if args.len()<= i+1 {
            return Err(ErrorFlags::InvalidArguments);
        }

        init.branch_main = args[i + 1].clone();

        Ok(i + 2)
    }

    fn add_file_config(
        init: &mut Init,
        i: usize,
        args: &[String],
    ) -> Result<usize, ErrorFlags> {
        if Self::is_flag(&args[i]) {
            return Err(ErrorFlags::WrongFlag);
        }
        if init.files .len() > 1 {
            return Err(ErrorFlags::InvalidArguments);
        }
        let path_aux = args[i].clone();
        let root = if path_aux.starts_with('/') {
            path_aux
        } else {
            let current_dir = env::current_dir().map_err(|_| ErrorFlags::InvalidArguments)?;
            let current_dir_display = current_dir.display();
            format!("{}/{}", current_dir_display, path_aux)
        };
        init.files.push(root);

        Ok(i + 1)
    }

    fn run(&self, output: &mut dyn Write) -> Result<(), ErrorFlags> {
        for path in &self.files {
            self.run_for_content(path, output)?;
        }
        Ok(())
    }

    fn run_for_content(&self, file : &String, output: &mut dyn Write) -> Result<(), ErrorFlags> {
        self.create_dirs(file)?;
        self.create_files(file)?;
        let output_text = format!("Initialized empty Git repository in {}", file);
        let _ = writeln!(output, "{}", output_text );
        Ok(())
    }

    fn create_dirs(&self, file : &String) -> Result<(), ErrorFlags> {
        if fs::create_dir_all(file).is_err(){
            return Err(ErrorFlags::InvalidArguments);
        }
        let file_aux = if !self.working_directory {
            file.clone()
        } else {
            format!("{}/.git", file)
        };
        self.create_dir(&file_aux,"objects".to_string())?;
        self.create_dir(&file_aux,"objects/info".to_string())?;
        self.create_dir(&file_aux,"objects/pack".to_string())?;
        self.create_dir(&file_aux,"refs".to_string())?;
        self.create_dir(&file_aux,"refs/tags".to_string())?;
        self.create_dir(&file_aux,"refs/heads".to_string())?;
        self.create_dir(&file_aux,"branches".to_string())?;
        Ok(())     
    }

    fn create_dir(&self, file: &String , name : String) -> Result<(), ErrorFlags> {
        if fs::create_dir_all(format!("{}/{}", file, name)).is_ok(){
            Ok(())
        } else {
            return Err(ErrorFlags::InvalidArguments);
        }
    }


    fn create_files(&self, file : &String) -> Result<(), ErrorFlags> {
        if fs::create_dir_all(file).is_err(){
            return Err(ErrorFlags::InvalidArguments);
        }
        let file_aux = if !self.working_directory {
            file.clone()
        } else {
            format!("{}/.git", file)
        };
        self.create_file(&file_aux,"HEAD".to_string())?;
        Ok(())     
    }

    fn create_file(&self, file: &String, name: String) -> Result<(), ErrorFlags> {
        if fs::create_dir_all(file).is_ok() {
            let mut archivo = match File::create(format!("{}/{}",file, name)) {
                Ok(mut archivo) => {
                    let texto = format!("ref: refs/heads/{}", self.branch_main);
                    let _: Result<(), ErrorFlags> = match archivo.write_all(texto.as_bytes()) {
                        Ok(_) => Ok(()),
                        Err(_) => Err(ErrorFlags::InvalidArguments),
                    };
                }
                Err(_) => return Err(ErrorFlags::InvalidArguments),
            };
        } else {
            return Err(ErrorFlags::InvalidArguments);
        }
    
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_nombre_incorrecto() {
        let mut logger = Logger::new(".git/logs").unwrap();
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args: &[String] = &[];
        assert!(matches!(
            Init::run_from("", args, &mut stdin_mock, &mut stdout_mock, &mut logger),
            Err(ErrorFlags::CommandName)
        ));
    }

    #[test]
    fn test_path_not_null() {
        let mut logger = Logger::new(".git/logs").unwrap();
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let relative_path = "/home/melina/Documents/mica".to_string();
        let absolute_path = "/home/melina/Documents/mica".to_string();
        let args: &[String] = &[relative_path];
        assert!(Init::run_from(
            "init",
            args,
            &mut stdin_mock,
            &mut stdout_mock,
            &mut logger
        )
        .is_ok());
        let Ok(output) = String::from_utf8(output_string) else {
            panic!("Error");
        };
        let aux = format!("Initialized empty Git repository in {}\n",absolute_path);
        assert_eq!(output, aux);
        _ = fs::remove_dir_all(format!("{}/.git", absolute_path));

    }

    #[test]
    fn test_path_null() {
        let mut logger = Logger::new(".git/logs").unwrap();
        let mut output_string = Vec::new();
        let mut stdout_mock = Cursor::new(&mut output_string);

        let input = "";
        let mut stdin_mock = Cursor::new(input.as_bytes());
        let relative_path = "/home/melina/Documents/cami".to_string();
        let absolute_path = "/home/melina/Documents/cami".to_string();
        let args: &[String] = &[relative_path];
        assert!(Init::run_from(
            "init",
            args,
            &mut stdin_mock,
            &mut stdout_mock,
            &mut logger
        )
        .is_ok());
        let Ok(output) = String::from_utf8(output_string) else {
            panic!("Error");
        };
        let aux = format!("Initialized empty Git repository in {}\n",absolute_path);
        assert_eq!(output, aux);
        _ = fs::remove_dir_all(format!("{}/.git", absolute_path));

    }
}
