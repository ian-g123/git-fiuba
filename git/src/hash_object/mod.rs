use crate::{command::Command, error_args::ErrorArgs};
use std::fmt;
enum Args {
    Type(TypeValues),
    Write,
    StdIn,
    Path(String),
}

enum TypeValues {
    Commit,
    Tree,
    Blob,
    Tag,
}

impl fmt::Display for Args {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Args::Type(type_value) => Ok(write!(f, "Type: {type_value}")?),
            Args::Write => Ok(write!(f, "write")?),
            Args::StdIn => Ok(write!(f, "stdin")?),
            Args::Path(path) => Ok(write!(f, "Type: {path}")?),
        }
    }
}

impl fmt::Display for TypeValues {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::Commit => Ok(write!(f, "commit")?),
            Self::Tree => Ok(write!(f, "tree")?),
            Self::Blob => Ok(write!(f, "blob")?),
            Self::Tag => Ok(write!(f, "tag")?),
        }
    }
}

impl Args {
    pub fn from_flag_and_values(flag: &str, values: Vec::<String>) -> Result<Args, ErrorArgs> {
        match flag {
            "-t" => create_flag_type(values),
            "-w" => create_flag_write(values),
            "--stdin" => create_flag_stdin(values),
            "--path" => create_flag_path(values),
            _ => Err(ErrorArgs::InvalidFlag),
        }
    }
}

fn create_flag_type(values: Vec<String>) -> Result<Args, ErrorArgs> {
    if values.len() != 1 {
        return Err(ErrorArgs::InvalidFlag);
    }
    match values.first().map(|s| s.as_str()) {
        Some("commit") => Ok(Args::Type(TypeValues::Commit)),
        Some("tree") => Ok(Args::Type(TypeValues::Tree)),
        Some("blob") => Ok(Args::Type(TypeValues::Blob)),
        Some("tag") => Ok(Args::Type(TypeValues::Tag)),
        _ => return Err(ErrorArgs::InvalidFlag),
    }
}

fn create_flag_write(values: Vec<String>) -> Result<Args, ErrorArgs> {
    if values.len() != 0 {
        return Err(ErrorArgs::InvalidFlag);
    }
    Ok(Args::Write)
}

fn create_flag_stdin(values: Vec<String>) -> Result<Args, ErrorArgs> {
    if values.len() != 0 {
        return Err(ErrorArgs::InvalidFlag);
    }
    Ok(Args::StdIn)
}

fn create_flag_path(values: Vec<String>) -> Result<Args, ErrorArgs> {
    if values.len() != 1 {
        return Err(ErrorArgs::InvalidFlag);
    }
    match values.first() {
        Some(path) => Ok(Args::Path(path.to_string())),
        _ => return Err(ErrorArgs::InvalidFlag),
    }
}

pub struct HashObject {}

impl Command for HashObject {
    fn run(name: &str, args: &[String]) -> Result<(), ErrorArgs> {
        if name != "hash-object" {
            print!("Nombre no hash");
            return Err(ErrorArgs::CommandName);
        }

        let mut recorded_values = Vec::<Args>::new();
        let mut current_flag = "";
        let mut values_buffer = Vec::<String>::new();

        for arg in args {
            if Self::is_flag(&arg) {
                if !current_flag.is_empty() {
                    match Args::from_flag_and_values(current_flag, values_buffer) {
                        Ok(value) => recorded_values.push(value),
                        Err(error) => return Err(error),
                    }
                }
                values_buffer = Vec::<String>::new();
                current_flag = arg;
            } else {
                values_buffer.push(arg.to_string());
            }
        }

        for value in recorded_values {
            println!("{}", value);
        }

        Ok(())
    }
}

// git hash-object -t blob --stdin -w  --path <file>
// [-t, blob, --stdin, -w, --path, <file>]

// -t: [blob]
// --stdin: []
// -w: []
// --path: [<file>]
