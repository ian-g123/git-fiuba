use super::flags::FlagsHashObject;
use crate::{commands::command::Command, error_args::ErrorFlags};

pub struct HashObject {}

// git hash-object -t blob --stdin -w  --path <file>

impl Command for HashObject {
    fn run(name: &str, args: &[String]) -> Result<(), ErrorFlags> {
        if name != "hash-object" {
            print!("El nombre no es hash-object");
            return Err(ErrorFlags::CommandName);
        }

        let mut recorded_values = Vec::<FlagsHashObject>::new();
        let mut current_flag = "";
        let mut values_buffer = Vec::<String>::new();

        for arg in args {
            if Self::is_flag(&arg) {
                if !current_flag.is_empty() {
                    match FlagsHashObject::get_flag(current_flag, values_buffer) {
                        Ok(value) => recorded_values.push(value),
                        Err(error) => return Err(ErrorFlags::InvalidFlag),
                    }
                }
                values_buffer = Vec::<String>::new();
                current_flag = arg;
            } else {
                values_buffer.push(arg.to_string());
            }
        }

        recorded_values[0].get

        // for value in recorded_values {
        //     println!("{}", value);
        // }

        Ok(())
    }
}
