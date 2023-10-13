// Se deben crear con 'git add' y agregar al Index con 'git commit'.

use std::path::PathBuf;

pub struct Change{
    path: PathBuf,
    hash: String,
    content: Vec<u8>
}

impl Change {
    fn new(path: String, hash:String){
        let path = PathBuf::from(path);
        
    }
}