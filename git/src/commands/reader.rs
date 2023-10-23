use std::io::{Read, Result};
use std::net::TcpStream;

pub struct TcpStreamBuffer<'a> {
    stream: &'a TcpStream,
    buffer: Vec<u8>,
    pos: usize,
}

impl<'a> TcpStreamBuffer<'a> {
    pub fn new(stream: &'a TcpStream) -> TcpStreamBuffer<'a> {
        TcpStreamBuffer {
            stream,
            buffer: Vec::new(),
            pos: 0,
        }
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
        self.pos = 0;
    }

    pub fn clean_up_to(&mut self, pos: usize) {
        self.buffer.drain(..pos);
        self.pos -= pos;
    }

    pub fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }

    pub fn get_pos(&self) -> usize {
        self.pos
    }
}

impl<'a> Read for TcpStreamBuffer<'a> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let num_bytes_to_read = buf.len();
        let mut num_bytes_read = 0;
        if self.pos + 1 < self.buffer.len() {
            let num_bytes_to_copy = std::cmp::min(num_bytes_to_read, self.buffer.len() - self.pos);
            buf[..num_bytes_to_copy]
                .copy_from_slice(&self.buffer[self.pos..self.pos + num_bytes_to_copy]);
            self.pos += num_bytes_to_copy;
            num_bytes_read += num_bytes_to_copy;
        }
        if (num_bytes_read < num_bytes_to_read) {
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

//impl<'a> Copy for TcpStreamBuffer<'a> {}
