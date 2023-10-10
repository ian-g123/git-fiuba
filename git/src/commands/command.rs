use crate::error_args::ErrorFlags;

pub trait Command {
    fn run(name: &str, args: &[String]) -> Result<(), ErrorFlags>;

    fn is_flag(arg: &str) -> bool {
        arg.starts_with('-')
    }
}
