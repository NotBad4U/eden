use std::net::{SocketAddr, TcpStream, TcpListener};
use std::io::{Read, Write, ErrorKind, BufReader};

//WIP
pub struct SystemNetworking {
    pub system_id: u8,
    sessions: Vec<Session>,
    system_addr: SocketAddr,
    remotes_systems_addr: Vec<SocketAddr>,
}

impl SystemNetworking {

    pub fn new(system_id: u8, system_addr: SocketAddr) -> Self {
        SystemNetworking {
            system_id,
            sessions: Vec::new(),
            system_addr,
            remotes_systems_addr: Vec::new(),
        }
    }

}

pub struct Session {
    stream: TcpStream,
    buf_read: BufReader<TcpStream>,
    buf_write: Vec<u8>,
    cursor_buf_write: usize,
}

impl Session {
    pub fn new(stream: TcpStream) -> Self {
        stream.set_nonblocking(true).unwrap();
        stream.set_nodelay(true).unwrap();
        let buf_read = BufReader::with_capacity(1024 * 1024, stream.try_clone().expect("can't clone"));

        Session {
            stream,
            buf_read,
            buf_write: Vec::new(),
            cursor_buf_write: 0,
        }
    }
}