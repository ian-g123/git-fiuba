use git_lib::objects::author::Author;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimplfiedAuthor {
    name: String,
    email: String,
    date: String,
}

impl SimplfiedAuthor {
    /// Recibe un Author y una fecha, y crea un SimplfiedAuthor a partir de esta información.
    pub fn from_author(get_author: Author, get_author_date: String) -> Self {
        SimplfiedAuthor {
            name: get_author.get_name(),
            email: get_author.get_email(),
            date: get_author_date,
        }
    }

    /// Convierte el SimplfiedAuthor en string
    pub fn to_string(&self) -> String {
        format!("{} <{}> {}", self.name, self.email, self.date)
    }
}
