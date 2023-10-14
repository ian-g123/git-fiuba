use super::{tree::Tree, blob::Blob};

#[derive(Debug, Clone)]
pub enum TreeOrBlob{
    TreeObject(Tree),
    BlobObject(Blob)
}

impl TreeOrBlob{
    pub fn get_hash(&self)-> String{
        match self{
            TreeOrBlob::BlobObject(blob) => blob.get_hash(),
            TreeOrBlob::TreeObject(tree) => tree.get_hash()
        }
    }
}