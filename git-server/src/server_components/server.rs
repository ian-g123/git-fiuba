use std::{io::Read, net::TcpListener, thread};

use git_lib::{command_errors::CommandError, server_components::pkt_strings::Pkt};

use crate::server_components::server_worker::ServerWorker;

pub struct Server {
    address: String,
    path: String,
    pub listener_handle: thread::JoinHandle<()>,
}

impl Server {
    pub fn start_server(address: &str, path: &str) -> Result<Server, CommandError> {
        println!("Starting server...");
        let listener = TcpListener::bind(address).map_err(|error| CommandError::Io {
            message: format!("No se pudo iniciar el servidor en la dirección {}", address),
            error: error.to_string(),
        })?;
        let path_str = path.to_string();
        let listener_handle = thread::spawn(move || {
            let mut worker_threads = vec![];
            for client_stream in listener.incoming() {
                let path = path_str.clone();
                let worker_thread = thread::spawn(move || {
                    let path = path.clone();
                    let mut worker = ServerWorker::new(path, client_stream.unwrap());
                    match worker.handle_connection() {
                        Ok(_) => println!("Connection handled successfully"),
                        Err(error) => eprintln!("{error}"),
                    }
                });
                worker_threads.push(worker_thread);
            }
        });

        Ok(Server {
            address: address.to_string(),
            path: path.to_string(),
            listener_handle: listener_handle,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File},
        io::Write,
        process::Command,
    };

    use super::*;

    #[test]
    #[ignore = "Needs server"]
    fn test_server() {
        let path = "./tests/data/server/test1";
        let git_bin = "../../../../../../../target/debug/git";
        instanciate_repo(path, git_bin);
        let address = "0.0.0.0:9418";

        match Server::start_server(address, "") {
            Ok(server) => {
                println!("Server started");
                server.listener_handle.join().unwrap();
            }
            Err(error) => eprintln!("{error}"),
        }
    }

    fn instanciate_repo(path: &str, git_bin: &str) {
        _ = fs::remove_dir_all(format!("{}/server-files/repo", path));
        _ = fs::remove_dir_all(format!("{}/client", path));

        let Ok(_) = fs::create_dir_all(path.clone()) else {
            panic!("No se pudo crear el directorio")
        };

        let Ok(_) = fs::create_dir_all(format!("{}/server-files/repo", path)) else {
            panic!("No se pudo crear el directorio")
        };

        assert!(
            Command::new(git_bin)
                .arg("init")
                .current_dir(path.to_owned() + "/server-files/repo")
                .status()
                .is_ok(),
            "No se pudo agregar el archivo testfile"
        );

        let mut file = File::create(path.to_owned() + "/server-files/repo/testfile").unwrap();
        file.write_all(b"contenido\n").unwrap();

        assert!(
            Command::new(git_bin)
                .arg("add")
                .arg("testfile")
                .current_dir(path.to_owned() + "/server-files/repo")
                .status()
                .is_ok(),
            "No se pudo agregar el archivo testfile"
        );

        assert!(
            Command::new(git_bin)
                .arg("commit")
                .arg("-m")
                .arg("hi")
                .current_dir(path.to_owned() + "/server-files/repo")
                .status()
                .is_ok(),
            "No se pudo hacer commit"
        );

        assert!(
            Command::new("touch")
                .arg("git-daemon-export-ok")
                .current_dir(path.to_owned() + "/server-files/repo/.git")
                .status()
                .is_ok(),
            "No se pudo crear el archivo testfile"
        );
    }
}
