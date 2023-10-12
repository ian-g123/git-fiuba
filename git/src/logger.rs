use std::{
    fs::{self, File},
    io::{Error, Write},
    path::Path,
    sync::mpsc::{channel, Sender},
    thread,
};

pub struct Logger {
    logs_sender: Option<Sender<String>>,
    writer_thread_handle: Option<thread::JoinHandle<()>>,
}

/// Guarda mensajes en un archivo de texto
impl Logger {
    /// Instancia logger para escribir en el archivo en path. Lo crea si no existe.
    pub fn new(path: &str) -> Result<Self, Error> {
        let path = Path::new(path);
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let mut file: File = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
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
    fn test_logger_single_line() {
        let test_dir = "test/data/logger/test1/";
        delete_directory_content(test_dir);
        let path = format!("{test_dir}.git/logs");

        let logger_result = Logger::new(&path);
        match logger_result {
            Ok(mut logger) => {
                let msg = "Hello, world!";
                logger.log(msg);
                assert!(Path::new(&path).exists());
                drop(logger);
                let Ok(output_content) = fs::read_to_string(path) else {
                    panic!("No se pudo leer archivo de salida")
                };
                assert_eq!(output_content, format!("{msg}\n"));
            }
            Err(error) => panic!("No se pudo crear logger: {}", error),
        };
    }

    #[test]
    fn test_logger_two_lines() {
        let test_dir = "test/data/logger/test2/";
        delete_directory_content(test_dir);
        let path = format!("{test_dir}.git/logs");

        let logger_result = Logger::new(&path);
        match logger_result {
            Ok(mut logger) => {
                let msg_1 = "Hello, world 1!";
                let msg_2 = "Hello, world 2!";
                logger.log(msg_1);
                logger.log(msg_2);
                assert!(Path::new(&path).exists());
                drop(logger);
                let Ok(output_content) = fs::read_to_string(path) else {
                    panic!("No se pudo leer archivo de salida")
                };
                assert_eq!(output_content, format!("{}\n{}\n", msg_1, msg_2));
            }
            Err(error) => panic!("No se pudo crear logger: {}", error),
        };
    }

    #[test]
    fn test_logger_open_existing_log_file() {
        let test_dir = "test/data/logger/test3/";
        delete_directory_content(test_dir);
        let path = format!("{test_dir}.git/logs");

        let logger_1_result = Logger::new(&path);
        let Ok(mut logger_1) = logger_1_result else {
            if let Err(error) = logger_1_result {
                panic!("No se pudo crear logger 1: {}", error)
            }
            panic!("No se pudo crear logger 1")
        };
        let msg_1 = "Hello, world 1!";
        logger_1.log(msg_1);
        drop(logger_1);

        let logger_2_result = Logger::new(&path);
        let Ok(mut logger_2) = logger_2_result else {
            if let Err(error) = logger_2_result {
                panic!("No se pudo crear logger 2: {}", error)
            }
            panic!("No se pudo crear logger 2")
        };
        let msg_2 = "Hello, world 2!";
        logger_2.log(msg_2);
        drop(logger_2);

        assert!(Path::new(&path).exists());
        let Ok(output_content) = fs::read_to_string(path) else {
            panic!("No se pudo leer archivo de salida")
        };

        assert_eq!(output_content, format!("{}\n{}\n", msg_1, msg_2));
    }

    fn delete_directory_content(path: &str) {
        let dir = Path::new(path);
        if dir.exists() {
            let _ = fs::remove_dir_all(dir);
        }
        let _ = fs::create_dir_all(dir);
    }
}
