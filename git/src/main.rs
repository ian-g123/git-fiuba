use git::logger::Logger;

fn main() {
    let Ok(mut logger) = Logger::new(".git/logs") else {
        return;
    };
    logger.log("Hello, world!");
}
