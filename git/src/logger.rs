use std::{
    fs::OpenOptions,
    io::{Error, Write},
    path::Path,
    sync::mpsc::{channel, Sender},
    thread,
};

pub struct Logger {
    logs_sender: Option<Sender<String>>,
    writer_thread_handle: Option<thread::JoinHandle<()>>,
}

impl Logger {
    /// Instancia logger para escribir en el archivo en path. Lo crea si no existe.
    pub fn new(path: &str) -> Result<Self, Error> {
        let path = Path::new(path);
        // If existent, opens file in append mode. If not, it creates it. If path doesnt exist, create it too
        let mut file: std::fs::File = OpenOptions::new().create(true).append(true).open(path)?;
        let (tx, rx) = channel::<String>();

        // create writer thread
        let handle = thread::spawn(move || loop {
            match rx.recv() {
                Ok(msg) => {
                    let _ = file.write_all(msg.as_bytes());
                }
                Err(_) => {
                    break;
                }
            }
        });

        Ok(Logger {
            logs_sender: Some(tx),
            writer_thread_handle: Some(handle),
        })
    }

    /// Escribe msg en el archivo de logs
    pub fn log(&mut self, msg: &str) {
        if let Some(sender) = &self.logs_sender {
            let _ = sender.send(msg.to_string());
            let _ = sender.send("\n".to_string());
        }
    }
}

impl Drop for Logger {
    fn drop(&mut self) {
        drop(self.logs_sender.take());

        if let Some(writer_thread) = self.writer_thread_handle.take() {
            let _ = writer_thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    #[ignore]
    fn test_logger_single_line() {
        let test_dir = "test/data/logger/.git/";
        delete_directory_content(test_dir);
        let path = format!("{test_dir}/logs_test_1");

        let logger_result = Logger::new(&path);
        match logger_result {
            Ok(mut logger) => {
                let msg = "Hello, world!";
                logger.log(msg);
                assert!(Path::new(&path).exists());
                // Espera a que logger termine de escribir
                drop(logger);
                let Ok(output_content) = fs::read_to_string(path) else {
                    panic!("Could not read output file")
                };
                assert_eq!(output_content, msg);
            }
            Err(error) => panic!("Could not create logger: {}", error),
        };
    }

    #[test]
    fn test_logger_two_lines() {
        let test_dir = "test/data/logger/.git/";
        delete_directory_content(test_dir);
        let path = format!("{test_dir}/logs_test_2");

        let logger_result = Logger::new(&path);
        match logger_result {
            Ok(mut logger) => {
                let msg_1 = "Hello, world 1!";
                let msg_2 = "Hello, world 2!";
                logger.log(msg_1);
                logger.log(msg_2);
                assert!(Path::new(&path).exists());
                // Espera a que logger termine de escribir
                drop(logger);
                let Ok(output_content) = fs::read_to_string(path) else {
                    panic!("Could not read output file")
                };
                assert_eq!(output_content, format!("{}\n{}\n", msg_1, msg_2));
            }
            Err(error) => panic!("Could not create logger: {}", error),
        };
    }

    fn delete_directory_content(path: &str) {
        let dir = Path::new(path);
        if dir.exists() {
            let _ = fs::remove_dir_all(dir);
        }
        let _ = fs::create_dir_all(dir);
    }
}
