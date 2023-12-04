use std::{
    fs::{self, File},
    io::Write,
    path::Path,
    process::Command,
    sync::mpsc::{channel, Sender},
    thread,
};

use crate::{command_errors::CommandError, logger_sender::LoggerSender};

#[derive(Debug)]
pub struct Logger {
    logs_sender: Option<Sender<String>>,
    writer_thread_handle: Option<thread::JoinHandle<()>>,
}

/// Guarda mensajes en un archivo de texto
impl Logger {
    /// Instancia logger para escribir en el archivo en path. Lo crea si no existe.
    pub fn new(path_name: &str) -> Result<Self, CommandError> {
        let path_name = path_name.to_string();
        let path = Path::new(&path_name);
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }

        let mut file: File = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|error| {
                CommandError::FileOpenError(format!("{}: {}", path_name, error.to_string()))
            })?;
        let (tx, rx) = channel::<String>();

        let handle = thread::spawn(move || {
            while let Ok(msg) = rx.recv() {
                let _ = file.write_all(msg.as_bytes());
            }
        });

        Ok(Logger {
            logs_sender: Some(tx),
            writer_thread_handle: Some(handle),
        })
    }

    pub fn new_dummy() -> Self {
        Logger {
            logs_sender: None,
            writer_thread_handle: None,
        }
    }

    /// Escribe msg en el archivo de logs
    pub fn log(&mut self, msg: &str) {
        if let Some(sender) = &self.logs_sender {
            let time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
            let _ = sender.send(format!("{}: {}\n", time, msg));
        }
    }

    pub fn get_logs_sender(&self) -> Result<LoggerSender, CommandError> {
        match self.logs_sender.clone() {
            Some(sender) => Ok(LoggerSender::new(sender)),
            None => Err(CommandError::NotValidLogger),
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
    use super::*;
    use std::fs;

    #[test]
    fn test_logger_single_line() {
        let test_dir = "tests/data/logger/test1/";
        delete_directory_content(test_dir);
        let path = format!("{test_dir}.git/logs.log");

        let mut logger = Logger::new(&path).unwrap();

        let msg = "Hello, world!";
        logger.log(msg);
        assert!(Path::new(&path).exists());

        drop(logger);
        let Ok(output_content) = fs::read_to_string(path) else {
            panic!("No se pudo leer archivo de salida")
        };
        let lines = output_content.lines().collect::<Vec<&str>>();
        assert_line(&lines, 0, msg);
    }

    #[test]
    fn test_logger_two_lines() {
        let test_dir = "tests/data/logger/test2/";
        delete_directory_content(test_dir);
        let path = format!("{test_dir}.git/logs.log");

        let mut logger = Logger::new(&path).unwrap();

        let msg_1 = "Hello, world 1!";
        let msg_2 = "Hello, world 2!";
        logger.log(msg_1);
        logger.log(msg_2);
        assert!(Path::new(&path).exists());

        drop(logger);
        let Ok(output_content) = fs::read_to_string(path) else {
            panic!("No se pudo leer archivo de salida")
        };

        let lines = output_content.lines().collect::<Vec<&str>>();
        assert_line(&lines, 0, msg_1);
        assert_line(&lines, 1, msg_2);
    }

    #[test]
    fn test_logger_open_existing_log_file() {
        let test_dir = "tests/data/logger/test3/";
        delete_directory_content(test_dir);
        let path = format!("{test_dir}.git/logs.log");

        let mut logger_1 = Logger::new(&path).unwrap();
        let msg_1 = "Hello, world 1!";
        logger_1.log(msg_1);
        drop(logger_1);

        let mut logger_2 = Logger::new(&path).unwrap();
        let msg_2 = "Hello, world 2!";
        logger_2.log(msg_2);
        drop(logger_2);

        assert!(Path::new(&path).exists());
        let Ok(output_content) = fs::read_to_string(path) else {
            panic!("No se pudo leer archivo de salida")
        };
        let lines = output_content.lines().collect::<Vec<&str>>();
        assert_line(&lines, 0, msg_1);
        assert_line(&lines, 1, msg_2);
    }

    fn delete_directory_content(path: &str) {
        let dir = Path::new(path);
        if dir.exists() {
            let _ = fs::remove_dir_all(dir);
        }
        let _ = fs::create_dir_all(dir);
    }

    fn assert_line(lines: &Vec<&str>, line_number: usize, expected_line: &str) {
        assert_eq!(
            &lines[line_number][lines[line_number].len() - expected_line.len()..],
            expected_line
        );
    }
}
