use std::fmt::{self, Display};

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
        let email = email_string.unwrap()[1..email_string.unwrap().len() - 1].to_string();

        let name = strings.join(" ");
        Ok(Author { name, email })
    }
}

impl fmt::Display for Author {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} <{}>", self.name, self.email)
    }
}
