use std::{
    fmt,
    io::{Read, Write},
};

use crate::commands::command_errors::CommandError;

use super::aux::*;

#[derive(Clone)]
pub struct Author {
    name: String,
    email: String,
}

impl Author {
    // Example string: Patricio Tourne Passarino <ptourne@fi.uba.ar>
    pub fn from_strings(strings: &mut Vec<&str>) -> Result<Self, CommandError> {
        let email_string = strings.pop();
        let email = match email_string {
            Some(email) => email[1..email.len() - 1].to_string(),
            None => return Err(CommandError::InvalidAuthor),
        };

        let name = strings.join(" ");
        Ok(Author { name, email })
    }

    pub fn new(name: &str, email: &str) -> Self {
        Author {
            name: name.to_string(),
            email: email.to_string(),
        }
    }

    pub(crate) fn write_to(&self, stream: &mut dyn Write) -> Result<(), CommandError> {
        self.name.write_to(stream)?;
        self.email.write_to(stream)?;
        Ok(())
    }

    pub(crate) fn read_from(stream: &mut dyn Read) -> Result<Self, CommandError> {
        let name = read_string_from(stream)?;
        let email = read_string_from(stream)?;
        Ok(Author { name, email })
    }
}

impl fmt::Display for Author {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} <{}>", self.name, self.email)
    }
}
