use super::error_flags::ErrorFlags;
use std::io::{Read, Write};

pub trait Command {
    fn run_from(
        name: &str,
        args: &[String],
        stdin: &mut dyn Read,
        output: &mut dyn Write,
    ) -> Result<(), ErrorFlags>;

    fn is_flag(arg: &str) -> bool {
        arg.starts_with('-')
    }
}
