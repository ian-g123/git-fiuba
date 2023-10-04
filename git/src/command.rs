use crate::error_args::ErrorArgs;

pub trait Command {
    fn run(name: &str, args: &[String]) -> Result<(), ErrorArgs>;

    fn is_flag(arg: &str) -> bool {
        arg.starts_with('-')
    }
}
