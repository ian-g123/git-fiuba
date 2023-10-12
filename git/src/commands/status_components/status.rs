use std::io::Write;
use std::io::Read;
use std::str;

use crate::commands::command::Command;
use crate::commands::error_flags::ErrorFlags;

pub struct Status {
    branch: bool,
    short: bool
}

impl Command for Status {
    fn run_from(
        name: &str,
        args: &[String],
        stdin: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), ErrorFlags> {
        if name != "status" {
            return Err(ErrorFlags::CommandName);
        }

        let instance = Self::new(args, output)?;

        //instance.run(stdin, output)?;
        Ok(())
    }
}

impl Status {
    fn new(args: &[String], output: &mut dyn Write) -> Result<Self, ErrorFlags> {
        if args.len() > 2 { //status -s -b (máximo)
            return Err(ErrorFlags::InvalidArguments);
        }
        let mut status = Status {
            branch: false,
            short: false
        };

        //status.config(args, output)?;

        Ok(status)
    }
}
/* 
    fn config(&mut self, args: &[String], output: &mut dyn Write) -> Result<(), ErrorFlags> {
        let mut current_flag = "";
        let mut values_buffer = Vec::<String>::new();

        for arg in args {
            if Self::is_flag(&arg) {
                if !current_flag.is_empty() {
                    self.add_flag(current_flag, &values_buffer, output)?;
                }
                values_buffer = Vec::<String>::new();
                current_flag = arg;
            } else {
                values_buffer.push(arg.to_string());
            }
        }
        Ok(())
    }

    fn is_flag(arg: &str) -> bool {
        arg.starts_with("-")
    }

    fn add_flag(
        &mut self,
        flag: &str,
        values: &Vec<String>,
        output: &mut dyn Write,
    ) -> Result<(), ErrorFlags> {
        let flags = [Self::add_type_flag, Self::add_stdin_flag];
        for f in flags.iter() {
            match f(self, flag, values, output) {
                Ok(_) => return Ok(()),
                Err(ErrorFlags::WrongFlag) => continue,
                Err(error) => return Err(error),
            }
        }
        Err(ErrorFlags::WrongFlag)
    }

    fn run(&self, stdin: &mut dyn Read, output: &mut dyn Write) -> Result<(), ErrorFlags> {
        let content: Vec<u8> = self.get_content(stdin)?;
        let header = self.get_header(&content);
        let mut data = Vec::new();

        data.extend_from_slice(header.as_bytes());
        data.extend_from_slice(&content);

        let hex_string = self.get_sha1(&data);
        write!(output, "{}", hex_string);
        Ok(())
    }

}

fn obtain_arguments(args: &[String]) -> Result<Vec<String>, ErrorFlags> {
    let mut arguments = Vec::new();
    for arg in args {
        if arg.starts_with("--") {
            arguments.push(arg.to_string());
        }
    }
    Ok(arguments)
}

*/
#[cfg(test)]
mod tests {
    use std::io::{Cursor, self};

    use super::*;

    #[test]
    fn create_status_fails_no_command(){
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);
        
        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args: &[String] = &[];
        assert!(matches!(
            Status::run_from("", args, &mut stdin_mock, &mut stdout_mock),
            Err(ErrorFlags::CommandName)
        ));
    }

    #[test]
    fn create_status_fails_wrong_command(){
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);
        
        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args: &[String] = &[];
        assert!(matches!(
            Status::run_from("hash-object", args, &mut stdin_mock, &mut stdout_mock),
            Err(ErrorFlags::CommandName)
        ));
    }

    #[test]
    fn create_status_fails_length(){
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);
        
        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args: &[String] = &["-b".to_string(), "-s".to_string(), "tercer argumento".to_string()];
        assert!(matches!(
            Status::run_from("status", args, &mut stdin_mock, &mut stdout_mock),
            Err(ErrorFlags::InvalidArguments)
        ));
    }

    #[test]
    fn create_status(){
        let mut output_string = Vec::new();
        let mut stdout_mock = io::Cursor::new(&mut output_string);
        
        let input = "prueba1";
        let mut stdin_mock = Cursor::new(input.as_bytes());

        let args: &[String] = &[];
        assert!(
            Status::run_from("status", args, &mut stdin_mock, &mut stdout_mock).is_ok());
    }

    
}
 