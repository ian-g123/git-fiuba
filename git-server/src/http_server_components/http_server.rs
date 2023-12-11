use std::{net::TcpListener, thread};

use git_lib::{command_errors::CommandError, logger::Logger};

use super::server_worker::ServerWorker;

pub struct HttpServer {
    pub listener_handle: thread::JoinHandle<Result<(), CommandError>>,
}

impl HttpServer {
    pub fn start_server(address: &str, path: &str) -> Result<HttpServer, CommandError> {
        println!("Starting HTTP server at {}...", address);
        let listener = TcpListener::bind(address).map_err(|error| CommandError::Io {
            message: format!("No se pudo iniciar el servidor en la dirección {}", address),
            error: error.to_string(),
        })?;
        let path_str = path.to_string();
        let listener_handle = thread::spawn(move || {
            let logger = match Logger::new("http-server-logs.log") {
                Ok(logger) => logger,
                Err(error) => {
                    return Err(CommandError::Io {
                        message: "No se pudo crear el archivo de logs".to_string(),
                        error: error.to_string(),
                    })
                }
            };
            let mut worker_threads = vec![];
            for client_stream in listener.incoming() {
                let client_stream = match client_stream {
                    Ok(client_stream) => client_stream,
                    Err(error) => {
                        return Err(CommandError::Io {
                            message: "Error obteniendo conexión".to_string(),
                            error: error.to_string(),
                        })
                    }
                };
                let path = path_str.clone();
                let logger_sender = match logger.get_logs_sender() {
                    Ok(logger_sender) => logger_sender,
                    Err(error) => {
                        return Err(CommandError::Io {
                            message: "Error obteniendo el sender del logger".to_string(),
                            error: error.to_string(),
                        })
                    }
                };
                let worker_thread = thread::spawn(move || {
                    println!("New connection");
                    let path = path.clone();
                    let mut worker = ServerWorker::new(path, client_stream, logger_sender);
                    worker.handle_connection()
                });
                worker_threads.push(worker_thread);
            }
            Ok(())
        });

        Ok(HttpServer { listener_handle })
    }
}
