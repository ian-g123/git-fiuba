pub trait FromPlain<'a>: serde::de::Deserialize<'a> + Sized {
    fn from_plain(socket: &mut std::net::TcpStream, len: usize) -> Self;
}
