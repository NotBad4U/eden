#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use agent_system::RemoteSystem;
use packet::Packet;

use std::net::{SocketAddr, TcpStream, TcpListener};
use std::io::{Read, Write, ErrorKind, BufReader};


pub struct NetworkingListener {
    cnt_id_system: u8,
    system_addr: SocketAddr,
    listener: TcpListener,
}

impl NetworkingListener {

    pub fn new(system_addr: SocketAddr, cnt_id_system: u8) -> Self {
        let listener = TcpListener::bind(system_addr).expect("can't listen remote system connection");
        listener.set_nonblocking(true).expect("Cannot set non-blocking");

        NetworkingListener {
            cnt_id_system,
            system_addr,
            listener,
        }
    }

    pub fn listen_incoming_remotes_systems_connection<M: Clone>(&mut self) -> Option<Vec<RemoteSystem<M>>> {
        let mut incoming_sessions = Vec::new();

        for stream in self.listener.incoming() {
            match stream {
                Ok(s) => {
                    incoming_sessions.push(RemoteSystem::Remote{
                        id: self.cnt_id_system,
                        session: Session::from_stream(s),
                    });
                    self.cnt_id_system += 1;
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    break;
                }
                Err(e) => panic!("encountered IO error: {}", e),
            }
        }

        if incoming_sessions.len() > 0 {
            Some(incoming_sessions)
        }
        else {
            None
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
    pub fn from_stream(stream: TcpStream) -> Self {
        //FIXME: Should return an error instead to panic
        stream.set_nonblocking(true).expect("Cannot set tcpstream nonblocking");
        let buf_read = BufReader::with_capacity(1024 * 1024, stream.try_clone().expect("can't clone"));

        Session {
            stream,
            buf_read,
            buf_write: Vec::new(),
            cursor_buf_write: 0,
        }
    }

    pub fn new(addr: SocketAddr) -> Self {
        //FIXME: Should return an error instead to panic
        let stream = TcpStream::connect(addr).expect("Cannot connect to addr");
        stream.set_nonblocking(true).expect("Cannot set tcpstream nonblocking");

        let buf_read = BufReader::with_capacity(1024 * 1024, stream.try_clone().expect("can't clone"));

        Session {
            stream,
            buf_read,
            buf_write: Vec::new(),
            cursor_buf_write: 0,
        }
    }

    pub fn enqueue<M: Clone>(&mut self, packet: &Packet<M>) {
        println!("enqueue");
    }

    pub fn try_send(&mut self) {
        loop {
            match self.stream.write(&self.buf_write[self.cursor_buf_write..]) {
                Ok(bytes_written) => {
                    if bytes_written > 0 {
                        self.cursor_buf_write += bytes_written;
                    } else {
                        break;
                    }
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) => panic!("{}", e),
            }
        }

        self.buf_write.drain(..self.cursor_buf_write);
        self.cursor_buf_write = 0;
    }

    //TODO: implem try_receive
    pub fn try_receive<M: Clone>() -> Option<Vec<M>> {
        None
    }
}