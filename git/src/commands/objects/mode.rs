#[derive(Clone, Debug)]
pub enum Mode{
    RegularFile = 100644,
    ExecutableFile = 100755,
    SymbolicLink = 120000,
    Submodule = 160000,
    Tree = 040000,

}