// Short format - changes types.
const SHORT_MODIFIED: &str = "M";
const SHORT_ADDED: &str = "A";
const SHORT_DELETED: &str = "D";
const SHORT_RENAMED: &str = "R";
const SHORT_UNTRACKED: &str = "??";
const SHORT_UNMODIFIED: &str = " ";

// Long format - changes types
const LONG_MODIFIED: &str = "modified";
const LONG_ADDED: &str = "new file";
const LONG_DELETED: &str = "deleted";
const LONG_RENAMED: &str = "renamed";

// const TYPE_CHANGED: &str = "T";

// const COPIED: &str = "C";
//const UPDATED: &str = "U";
//

#[derive(Clone)]
pub enum ChangeType {
    Modified,
    Unmodified,
    //TypeChanged,
    Added,
    Deleted,
    Renamed,
    //Updated,
    Untracked,
}

impl ChangeType {
    pub fn get_long_type(&self) -> String {
        match self {
            ChangeType::Modified => LONG_MODIFIED.to_string(),
            ChangeType::Deleted => LONG_DELETED.to_string(),
            ChangeType::Renamed => LONG_RENAMED.to_string(),
            _ => "".to_string(),
        }
    }
    pub fn get_short_type(&self) -> String {
        match self {
            ChangeType::Modified => SHORT_MODIFIED.to_string(),
            ChangeType::Unmodified => SHORT_UNMODIFIED.to_string(),
            ChangeType::Deleted => SHORT_DELETED.to_string(),
            ChangeType::Renamed => SHORT_RENAMED.to_string(),
            ChangeType::Added => SHORT_ADDED.to_string(),
            _ => "".to_string(),
        }
    }
}
