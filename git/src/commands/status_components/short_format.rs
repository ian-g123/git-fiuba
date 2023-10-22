use std::fmt::Write;

use super::format::Format;

pub struct ShortFormat;

impl Format for ShortFormat {
    fn get_status(output: &mut dyn Write) {}
}
