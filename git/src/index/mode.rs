#[derive(Clone)]
pub enum Mode{
    RegularFile,
    ExecutableFile,
    SymbolicLink,
    Tree,
}

impl Mode{
    pub fn get_mode(&self)-> usize{
        match self{
            Mode::RegularFile => 100644,
            Mode::ExecutableFile => 100755,
            Mode::SymbolicLink => 120000,
            Mode::Tree => 040000,
        }
    }
}