// Short format - changes types.
const SHORT_MODIFIED: &str = "M";
const SHORT_ADDED: &str = "A";
const SHORT_DELETED: &str = "D";
const SHORT_RENAMED: &str = "R";
const SHORT_UNTRACKED: &str = "?";
const SHORT_UNMODIFIED: &str = " ";
const SHORT_UNMERGED: &str = "U";

// Long format - changes types
const LONG_MODIFIED: &str = "modified";
const LONG_ADDED: &str = "new file";
const LONG_DELETED: &str = "deleted";
const LONG_RENAMED: &str = "renamed";

// merge format
const LONG_MERGE_DELETED_BY_THEM: &str = "deleted by them";
const LONG_MERGE_DELETED_BY_US: &str = "deleted by us";
const LONG_MERGE_ADDED_BY_BOTH: &str = "both added";
const LONG_MERGE_MODIFIED_BY_BOTH: &str = "both modified";

#[derive(Clone)]
pub enum ChangeType {
    Modified,
    ModifiedByBoth,
    Unmodified,
    Added,
    AddedByBoth,
    Deleted,
    DeletedByThen,
    DeletedByUs,

    Renamed,
    Untracked,
    Unmerged,
}

impl ChangeType {
    pub fn get_long_type(&self) -> String {
        match self {
            ChangeType::Modified => LONG_MODIFIED.to_string(),
            ChangeType::Deleted => LONG_DELETED.to_string(),
            ChangeType::Renamed => LONG_RENAMED.to_string(),
            ChangeType::Added => LONG_ADDED.to_string(),
            ChangeType::DeletedByThen => LONG_MERGE_DELETED_BY_THEM.to_string(),
            ChangeType::DeletedByUs => LONG_MERGE_DELETED_BY_US.to_string(),
            ChangeType::AddedByBoth => LONG_MERGE_ADDED_BY_BOTH.to_string(),
            ChangeType::ModifiedByBoth => LONG_MERGE_MODIFIED_BY_BOTH.to_string(),

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
            ChangeType::Untracked => SHORT_UNTRACKED.to_string(),
            ChangeType::Unmerged => SHORT_UNMERGED.to_string(),
            _ => "".to_string(),
        }
    }
}
