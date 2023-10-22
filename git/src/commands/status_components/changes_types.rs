// Short format - changes types.
const MODIFIED: &str = "M";
const UNMODIFIED: &str = " ";
// const TYPE_CHANGED: &str = "T";
const ADDED: &str = "A";
const DELETED: &str = "D";
const RENAMED: &str = "R";
// const COPIED: &str = "C";
//const UPDATED: &str = "U";
const UNTRACKED: &str = "??";

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
