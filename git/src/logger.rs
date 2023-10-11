use std::{
    fs::OpenOptions,
    io::{Error, Write},
    path::Path,
    sync::mpsc::{channel, Sender},
    thread,
};

pub struct Logger {
    logging_channel: Sender<String>,
    writer_thread: thread::JoinHandle<()>,
}

impl Logger {
    /// Instanciates logger to write in /.git/loggs
    pub fn new(path: &str) -> Result<Self, Error> {
        let path = Path::new(path);
        // If existent, opens file in append mode. If not, it creates it. If path doesnt exist, create it too
        let mut file: std::fs::File = OpenOptions::new().create(true).append(true).open(path)?;
        let (tx, rx) = channel::<String>();

        // create writer thread
        let handle = thread::spawn(move || {
            for msg in rx {
                file.write_all(msg.as_bytes());
            }
        });

        Ok(Logger {
            logging_channel: tx,
            writer_thread: handle,
        })
    }

    pub fn log(&mut self, msg: &str) {
        self.logging_channel.send(msg.to_string());
    }

    pub fn terminate(self) {
        let writer_thread = self.writer_thread;
        let _ = writer_thread.join();
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    // Run test of logger with workdir src/test/logger/
    #[test]
    fn test_logger() {
        let path = "test/logger/.git/logs";
        //delete logs file if exists

        let logger_result = Logger::new(path);
        match logger_result {
            Ok(mut logger) => {
                logger.log("Hello, world!");
                assert!(Path::new("test/logger/.git/logs").exists());
            }
            Err(error) => panic!("Could not create logger: {}", error),
        };
    }

    fn limpiar_directorio_program_output(path: &str) {
        let Ok(archivos) = fs::read_dir(path) else {
            return;
        };

        for archivo in archivos {
            let Ok(archivo) = archivo else {
                return;
            };
            let archivo_path = archivo.path();

            if archivo_path.is_file() {
                fs::remove_file(&archivo_path).ok();
            }
        }
    }
}
