
use super::error_flags::ErrorFlags;
use std::io::{self, Write};

pub trait Command {
    fn run_from(name: &str, args: &[String], output: &mut dyn Write) -> Result<(), ErrorFlags>;

    fn is_flag(arg: &str) -> bool {
        arg.starts_with('-')
    }
}
