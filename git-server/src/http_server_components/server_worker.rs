use std::{
    io::{BufRead as _, BufReader, Write},
    net::TcpStream,
};

use git_lib::{command_errors::CommandError, logger_sender::LoggerSender};

pub struct ServerWorker {
    path: String,
    socket: TcpStream,
    process_id: String,
    thread_id: String,
    logger_sender: LoggerSender,
}

impl ServerWorker {
    pub fn new(path: String, stream: TcpStream, logger_sender: LoggerSender) -> Self {
        let process_id = format!("{:?}", std::process::id());
        let thread_id = format!("{:?}", std::thread::current().id());
        Self {
            path,
            socket: stream,
            process_id,
            thread_id,
            logger_sender,
        }
    }

    fn log(&mut self, message: &str) {
        let time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        self.logger_sender.log(&format!(
            "[{}:{}] {}: {}",
            self.process_id, self.thread_id, time, message
        ));
    }

    pub fn handle_connection(&mut self) {
        self.log("New connection");
        match self.handle_connection_priv() {
            Ok(_) => self.log("Connection handled successfully"),
            Err(error) => {
                self.log(&format!("❌ Error: {}", error));
                eprintln!("{error}")
            }
        }
    }

    fn handle_connection_priv(&mut self) -> Result<(), CommandError> {
        let buf_reader = BufReader::new(&mut self.socket);
        let http_request: Vec<_> = buf_reader
            .lines()
            .map(|result| result.unwrap())
            .take_while(|line| !line.is_empty())
            .collect();
        self.log(&format!(
            "HTTP request received:\n{}",
            http_request.join("\n")
        ));
        let response = "HTTP/1.1 200 OK\r\n\r\n";

        self.socket.write_all(response.as_bytes()).unwrap();
        Ok(())
    }
}
