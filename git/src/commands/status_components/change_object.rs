use super::{changes_types::ChangeType, working_tree};

#[derive(Clone)]
pub struct ChangeObject {
    path: String,
    working_tree_status: ChangeType,
    staging_area_status: ChangeType,
}

impl ChangeObject {
    pub fn new(
        path: String,
        working_tree_status: ChangeType,
        staging_area_status: ChangeType,
    ) -> Self {
        ChangeObject {
            path: path,
            working_tree_status: working_tree_status,
            staging_area_status: staging_area_status,
        }
    }

    pub fn to_string_change(&self) -> String {
        let working_tree_status = self.working_tree_status.get_short_type();
        let staging_area_status = self.staging_area_status.get_short_type();
        format!(
            "{}{} {}",
            staging_area_status, working_tree_status, self.path
        )
    }
}
