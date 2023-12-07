use std::{collections::HashMap, fs};

use git_lib::command_errors::CommandError;
use git_server::http_server_components::http_server::HttpServer;
use git_server::tcp_server_components::tcp_server::TcpServer;

fn main() {
    let path = "server.config";
    let mut config = HashMap::<String, u32>::new();
    _ = config.insert("http_port".to_string(), 8080);
    _ = config.insert("tcp_port".to_string(), 9418);
    if let Ok(file) = fs::read_to_string(path) {
        let lines = file.lines();
        for (i, line) in lines.enumerate() {
            let Some((key, value)) = line.split_once('=') else {
                eprintln!("Failed to parse line {}: '{}'", i, line);
                continue;
            };
            let key = key.trim();
            let value = value.trim();
            if let Ok(value) = value.parse::<u32>() {
                config.insert(key.to_string(), value);
            } else {
                eprintln!("Failed to parse line {}: '{}'", i, line);
                continue;
            }
        }
    } else {
        eprintln!("Failed to read config file. Using defaults");
    }

    let Some(tcp_port) = config.get("tcp_port") else {
        unreachable!();
    };
    let tpc_address = format!("0.0.0.0:{}", tcp_port);
    let Some(http_port) = config.get("http_port") else {
        unreachable!();
    };
    let http_address = format!("0.0.0.0:{}", http_port);

    let tcp_server_result =
        TcpServer::start_server(&tpc_address, "").map_err(|error| CommandError::Io {
            message: "Error iniciando el servidor tcp".to_string(),
            error: error.to_string(),
        });

    let http_server_result =
        HttpServer::start_server(&http_address, "").map_err(|error| CommandError::Io {
            message: "Error iniciando el servidor http".to_string(),
            error: error.to_string(),
        });

    let tcp_server_opt = match tcp_server_result {
        Ok(tcp_server) => Some(tcp_server),
        Err(error) => {
            eprintln!("{}", error);
            None
        }
    };
    let http_server_opt = match http_server_result {
        Ok(http_server) => Some(http_server),
        Err(error) => {
            eprintln!("{}", error);
            None
        }
    };

    if let Some(tcp_server) = tcp_server_opt {
        if let Err(error) = tcp_server
            .listener_handle
            .join()
            .expect("Error joining tcp server thread")
        {
            eprintln!("{}", error);
        };
    }

    if let Some(http_server) = http_server_opt {
        if let Err(error) = http_server
            .listener_handle
            .join()
            .expect("Error joining http server thread")
        {
            eprintln!("{}", error);
        };
    }
}
