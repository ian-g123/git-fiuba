use std::fmt::{self};

use crate::commands::command_errors::CommandError;

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
}

impl fmt::Display for Author {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} <{}>", self.name, self.email)
    }
}
