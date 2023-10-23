// Short format - changes types.
const SHORT_MODIFIED: &str = "M";
const SHORT_ADDED: &str = "A";
const SHORT_DELETED: &str = "D";
const SHORT_RENAMED: &str = "R";
const SHORT_UNTRACKED: &str = "??";

// Long format - changes types
const Long_MODIFIED: &str = "modified";
const Long_ADDED: &str = "new file";
const Long_DELETED: &str = "deleted";
const Long_RENAMED: &str = "renamed";

// const TYPE_CHANGED: &str = "T";

// const COPIED: &str = "C";
//const UPDATED: &str = "U";
//const SHORT_UNMODIFIED: &str = " ";

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
