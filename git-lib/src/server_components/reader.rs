use std::io::Read;

use crate::command_errors::CommandError;

pub struct BuffedReader<'a> {
    stream: &'a mut dyn Read,
    buffer: Vec<u8>,
    pos: usize,
}

impl<'a> BuffedReader<'a> {
    pub fn new(stream: &'a mut dyn Read) -> BuffedReader<'a> {
        BuffedReader {
            stream,
            buffer: Vec::new(),
            pos: 0,
        }
    }

    pub fn clean_up_to_pos(&mut self) {
        self.buffer.drain(..self.pos);
        self.pos = 0;
    }

    // If the buffer is smaller than the given size, read from the stream until the buffer is at least the given size.
    pub fn record_and_fast_foward_to(&mut self, pos: usize) -> Result<(), CommandError> {
        let len = self.buffer.len();
        if len < pos {
            let bytes_to_read = pos - len;
            println!("recording {} bytes", bytes_to_read);
            let mut buffer = vec![0; bytes_to_read];
            self.read_exact(&mut buffer).map_err(|error| {
                CommandError::FileReadError(format!("Error al leer con buffed reader: {error}"))
            })?;
        }
        self.pos = pos;
        Ok(())
    }

    pub fn get_pos(&self) -> usize {
        self.pos.clone()
    }
}

impl<'a> Read for BuffedReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let num_bytes_to_read = buf.len();
        let mut num_bytes_read = 0;
        if self.pos + 1 < self.buffer.len() {
            let num_bytes_to_copy = std::cmp::min(num_bytes_to_read, self.buffer.len() - self.pos);
            buf[..num_bytes_to_copy]
                .copy_from_slice(&self.buffer[self.pos..self.pos + num_bytes_to_copy]);
            self.pos += num_bytes_to_copy;
            num_bytes_read += num_bytes_to_copy;
        }
        if num_bytes_read < num_bytes_to_read {
            let num_bytes_to_read_from_stream = num_bytes_to_read - num_bytes_read;
            let mut buffer = vec![0; num_bytes_to_read_from_stream];
            let num_bytes_read_from_stream = self.stream.read(&mut buffer)?;
            buf[num_bytes_read..num_bytes_read + num_bytes_read_from_stream]
                .copy_from_slice(&buffer[..num_bytes_read_from_stream]);
            num_bytes_read += num_bytes_read_from_stream;
            self.buffer
                .append(&mut buffer[..num_bytes_read_from_stream].to_owned());
            self.pos += num_bytes_read;
        }
        Ok(num_bytes_read)
    }
}
