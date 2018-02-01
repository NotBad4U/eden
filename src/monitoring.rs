#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use std::net::{SocketAddr, UdpSocket};

//WIP
pub struct Monitoring {
    pub system_id: u8,
    monitor_addr: SocketAddr,
    socket: UdpSocket,
}

impl Monitoring {

    fn new(system_id: u8, monitor_addr: SocketAddr) -> Self {
        if let Ok(socket) = UdpSocket::bind(monitor_addr) {
            Monitoring {
                system_id,
                monitor_addr: monitor_addr.into(),
                socket,
            }
        }
        else {
            //FIXME: Little brutal. Maybe we should retry to connect
            panic!("Can't connect to the remote monitoring system");
        }
    }

    fn aggregate_metrics(&mut self) {}

    fn send_metrics(&self) {}
}