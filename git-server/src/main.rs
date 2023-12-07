use git_server::server_components::server::Server;

fn main() {
    let address = "0.0.0.0:9418";

    match Server::start_server(address, "") {
        Ok(server) => {
            println!("Server started");
            server.listener_handle.join().unwrap().unwrap();
        }
        Err(error) => eprintln!("{error}"),
    }
}
