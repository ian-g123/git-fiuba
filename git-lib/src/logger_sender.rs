use std::sync::mpsc::Sender;

pub struct LoggerSender {
    logs_sender: Sender<String>,
}

/// Guarda mensajes en un archivo de texto
impl LoggerSender {
    pub fn new(logs_sender: Sender<String>) -> Self {
        Self { logs_sender }
    }

    /// Escribe msg en el archivo de logs
    pub fn log(&mut self, msg: &str) {
        let _ = self.logs_sender.send(msg.to_string());
        let _ = self.logs_sender.send("\n".to_string());
    }
}
