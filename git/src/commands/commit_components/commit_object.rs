#[derive(Clone)]
pub struct Commit{
    header:String,
    hash: String,
    mode: Mode,
    parent: Box<Commit>, //cambiar en Merge (puede tener varios padres),
    message: String,
    author: Author,
    date: String, //cambiar por Date o TimeStamp
}

#[derive(Clone)]
pub struct Author{
    name: String,
    email: String,
}

pub struct Tree{
    header: String,
    hash: String,
    mode: Mode,
    //objects: 
}

#[derive(Clone)]
pub enum Mode{
    RegularFile = 100644,
    ExecutableFile = 100755,
    SymbolicLink = 120000,
    Submodule = 160000,
    Tree = 040000,

}

