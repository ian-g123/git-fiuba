use crate::{
    command_errors::CommandError, logger::Logger, logger_sender::LoggerSender,
    objects_database::ObjectsDatabase,
};
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::TcpStream,
};

use super::{
    packfile_functions::read_objects_from_packfile, packfile_object_type::PackfileObjectType,
    pkt_strings::Pkt, reader::BuffedReader,
};

pub struct GitServer {
    pub socket: TcpStream,
    logger_sender: LoggerSender,
}

impl GitServer {
    pub fn connect_to(address: &str, logger: &mut Logger) -> Result<GitServer, CommandError> {
        let socket = TcpStream::connect(address).map_err(|error| {
            CommandError::Connection(format!(
                "No se pudo conectar al servidor en la dirección {}",
                error
            ))
        })?;
        Ok(GitServer {
            socket,
            logger_sender: logger.get_logs_sender()?,
        })
    }

    fn log(&mut self, message: &str) {
        let time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        self.logger_sender.log(&format!("{}: {}", time, message));
    }

    /// envía un mensaje al servidor y devuelve la respuesta
    fn send(&mut self, line: &str) -> Result<Vec<String>, CommandError> {
        let line = line.to_string().to_pkt_format();
        self.write_string_to_socket(&line)?;
        let lines = get_response_fn(&self.socket)?;
        Ok(lines)
    }

    /// explora el repositorio remoto y devuelve el hash del commit del branch head
    /// y un hashmap con: hash del commit -> nombre de la referencia
    pub fn explore_repository_upload_pack(
        &mut self,
        repository_path: &str,
        host: &str,
    ) -> Result<(String, HashMap<String, String>), CommandError> {
        let line = format!(
            "git-upload-pack {}\0host={}\0\0version=1\0\n",
            repository_path, host
        );
        let mut lines = self.send(&line)?;
        let first_line = lines.remove(0);
        if first_line != "version 1\n" {
            return Err(CommandError::ErrorReadingPktVerbose(format!(
                "Error al leer la versión del protocolo: {}",
                first_line
            )));
        }
        let head_branch_line = lines.remove(0);
        let Some((head_branch_commit, _)) = head_branch_line.split_once(' ') else {
            return Err(CommandError::ErrorReadingPktVerbose(format!(
                "Error al leer la rama de cabecera: {}",
                head_branch_line
            )));
        };
        let mut refs = HashMap::<String, String>::new();
        for line in lines {
            let (hash, ref_name) = line
                .split_once(' ')
                .ok_or(CommandError::ErrorReadingPktVerbose(format!(
                    "Error separando ref y hash: {}",
                    line
                )))
                .map(|(sha1, ref_name)| (sha1.trim().to_string(), ref_name.trim().to_string()))?;
            refs.insert(hash, ref_name);
        }
        Ok((head_branch_commit.to_string(), refs))
    }

    /// envía un mensaje al servidor para que envíe los objetos del repositorio
    /// y devuelve un vector con tuplas que contienen:\
    /// `(tipo de objeto, tamaño del objeto, contenido del objeto en bytes)`
    pub fn fetch_objects(
        &mut self,
        wants_commits: Vec<String>,
        haves_commits: Vec<String>,
        db: &ObjectsDatabase,
        logger: &mut Logger,
    ) -> Result<HashMap<[u8; 20], (PackfileObjectType, usize, Vec<u8>)>, CommandError> {
        logger.log("fetch_objects");
        let mut lines = Vec::<String>::new();
        for want_commit in wants_commits {
            let line = format!("want {}\n", want_commit);
            logger.log(&format!("Sending: {}", line));
            self.write_in_tpk_to_socket(&line)?;
        }
        self.write_string_to_socket("0000")?;
        if !haves_commits.is_empty() {
            for have in haves_commits {
                let line = format!("have {}\n", have);
                logger.log(&format!("Sending: {}", line));
                self.write_in_tpk_to_socket(&line)?;
            }
            self.write_string_to_socket("0000")?;
        }
        self.write_in_tpk_to_socket("done\n")?;
        logger.log("reading objects");

        match String::read_pkt_format(&mut self.socket)? {
            Some(line) => {
                logger.log(&format!("pushing: {:?}", line));
                lines.push(line);
            }
            None => {
                return Err(CommandError::ErrorReadingPktVerbose(format!(
                    "fetch_objects leyó un flush-pkt"
                )))
            }
        }
        logger.log(
            "Reading objects"
        );
        read_objects_from_packfile(&mut self.socket, db, logger)
    }

    fn write_string_to_socket(&mut self, line: &str) -> Result<(), CommandError> {
        self.log(&format!("⏫: {:?}", line));
        let message = line.as_bytes().to_owned();
        self.write_to_socket(&message)?;
        Ok(())
    }

    pub fn send_packfile(&mut self, packfile: &Vec<u8>) -> Result<(), CommandError> {
        self.log(&format!("⏫: [PACKFILE]"));

        self.write_to_socket(packfile)?;
        Ok(())
    }

    pub fn write_to_socket(&mut self, message: &Vec<u8>) -> Result<(), CommandError> {
        self.socket
            .write_all(message)
            .map_err(|error| CommandError::SendingMessage(error.to_string()))?;
        Ok(())
    }

    fn write_in_tpk_to_socket(&mut self, line: &str) -> Result<(), CommandError> {
        let line = line.to_string().to_pkt_format();

        self.write_string_to_socket(&line)
    }

    /// explora el repositorio remoto y devuelve el hash del commit del branch head
    /// y un hashmap con: nombre de la referencia -> hash del commit
    pub fn explore_repository_receive_pack(
        &mut self,
        repository_path: &str,
        host: &str,
    ) -> Result<HashMap<String, String>, CommandError> {
        let line = format!(
            "git-receive-pack {}\0host={}\0\0version=1\0\n",
            repository_path, host
        );

        let mut lines = self.send(&line)?;

        let mut refs_hash = HashMap::<String, String>::new();

        let _version = lines.remove(0);
        let first_line = lines.remove(0);

        let (hash, mut branch_name_and_options) =
            first_line
                .split_once(' ')
                .ok_or(CommandError::ErrorReadingPktVerbose(format!(
                    "Error al leer la rama de cabecera: {}",
                    first_line
                )))?;
        branch_name_and_options = &branch_name_and_options[11..branch_name_and_options.len() - 1]; // refs/heads/*\n
        let (branch_name, _options) = branch_name_and_options.split_once('\0').ok_or(
            CommandError::ErrorReadingPktVerbose(format!(
                "Error al leer las opciones: {}",
                branch_name_and_options
            )),
        )?;
        refs_hash.insert(branch_name.to_string(), hash.to_string());

        for line in &lines {
            let (hash, mut ref_name) = line.split_once(' ').ok_or(CommandError::ErrorReadingPkt)?;
            ref_name = &ref_name[11..ref_name.len() - 1]; // refs/heads/*\n
            refs_hash.insert(ref_name.to_string(), hash.to_string());
        }

        Ok(refs_hash)
    }

    pub fn negociate_recieve_pack(
        &mut self,
        hash_branch_status: HashMap<String, (String, String)>, // HashMap<branch, (nuevo_hash, viejo_hash)>
    ) -> Result<(), CommandError> {
        for (branch, (new_hash, old_hash)) in hash_branch_status {
            let line = format!("{} {} refs/heads/{}\n", new_hash, old_hash, branch);

            self.write_in_tpk_to_socket(&line).map_err(|_| {
                return CommandError::SendingMessage(
                    "Error al enviar el hash del branch".to_string(),
                );
            })?;
        }
        self.write_string_to_socket("0000").map_err(|_| {
            return CommandError::SendingMessage("Error al enviar 0000 al servidor".to_string());
        })?;
        return Ok(());
    }

    pub fn get_response(&mut self) -> Result<Vec<String>, CommandError> {
        let mut lines = Vec::<String>::new();
        loop {
            match String::read_pkt_format(&mut self.socket)? {
                Some(line) => {
                    println!("<=: {:?}", line);
                    lines.push(line);
                }
                None => break,
            }
        }
        Ok(lines)
    }
    pub fn just_read(&mut self) -> Result<Vec<u8>, CommandError> {
        let mut buf = Vec::new();
        self.socket.read(&mut buf).unwrap();
        Ok(buf)
    }
}

fn get_response_fn(mut socket: &TcpStream) -> Result<Vec<String>, CommandError> {
    let mut lines = Vec::<String>::new();
    loop {
        match String::read_pkt_format(&mut socket)? {
            Some(line) => {
                lines.push(line);
            }
            None => break,
        }
    }
    Ok(lines)
}
