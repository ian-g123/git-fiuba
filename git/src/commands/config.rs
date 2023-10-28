use git_lib::command_errors::CommandError;
use std::collections::HashMap;

pub struct Config {
    entries: HashMap<String, String>,
}
// Example file
// key1=value1
// key2=value2
impl Config {
    pub fn open() -> Result<Self, CommandError> {
        let mut entries: HashMap<String, String> = HashMap::new();
        let Ok(mut file) = std::fs::File::open(".git/config") else {
            return Ok(Self::default_config());
        };
        return Ok(Self::default_config());
        Err(CommandError::FileReadError("config".to_string()))
    }

    fn default_config() -> Config {
        let mut entries: HashMap<String, String> = HashMap::new();
        entries.insert("user.email".to_string(), "example@email.org".to_string());
        entries.insert("user.name".to_string(), "Foo Bar".to_string());
        Self { entries }
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.entries.get(key)
    }
}
