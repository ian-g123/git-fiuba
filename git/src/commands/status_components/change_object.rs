use super::changes_types::ChangeType;

#[derive(Clone)]
pub struct ChangeObject{
    hash: String,
    working_tree_status: Option<ChangeType>,
    staging_area_status: Option<ChangeType>,
}

impl ChangeObject{
    pub fn new(hash: String, working_tree_status: Option<ChangeType>,
        staging_area_status: Option<ChangeType>)->Self{
            ChangeObject { hash: hash, working_tree_status: working_tree_status, staging_area_status: staging_area_status }
        }
}