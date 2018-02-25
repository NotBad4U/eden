use zmq::{Socket, Context, SUB, PollItem, POLLIN, poll as zmq_poll, Message};

use packet::{Packet, Payload, BROADCAST_TO_ALL, BROADCAST_TO_SYSTEM, SEND_TO_AGENT};

use std::sync::mpsc::Receiver;
use std::net::SocketAddr;
use std::sync::Arc;

const NONBLOCKING_POLL: i64 = 0;

pub struct Collector<M: Payload> {
    system_id: u8,
    zmq_ctx: Arc<Context>,
    local_collector: Receiver<Packet<M>>,
    remotes_collector: Vec<Socket>,
}

impl <M: Payload>Collector<M> {

    pub fn new(system_id: u8,
        zmq_ctx: Arc<Context>,
        local_collector: Receiver<Packet<M>>,
    ) -> Self {
        Collector {
            system_id,
            zmq_ctx,
            local_collector,
            remotes_collector: Vec::new(),
        }
    }

    pub fn add_remote_collector(&mut self, addr: SocketAddr) {
        if let Ok(zmq_subscriber) = self.zmq_ctx.socket(SUB) {
            if let Ok(_) = zmq_subscriber.connect(&format!("tcp://{}", addr)) {
                zmq_subscriber.set_subscribe(format!("{}", self.system_id).as_bytes())
                    .and_then(|_| zmq_subscriber.set_subscribe(&[SEND_TO_AGENT, self.system_id]))
                    .and_then(|_| {
                        zmq_subscriber.set_subscribe(&[BROADCAST_TO_SYSTEM, self.system_id])
                    })
                    .and_then(|_| {
                        zmq_subscriber.set_subscribe(&[BROADCAST_TO_ALL])
                    })
                    .unwrap_or_else(|_| {
                        error!("Can't set message filters for the system: {}", self.system_id);
                    });

                self.remotes_collector.push(zmq_subscriber);
                info!("Listening the remote system {}", addr);
            };
        }
    }

    pub fn collect_packets(&self) -> Option<Vec<Packet<M>>> {
        let mut packets = Vec::new();
        packets = self.collect_remotes_packet(packets);
        packets = self.collect_local_packet(packets);

        if packets.len() > 0 {
            Some(packets)
        }
        else {
            None
        }
    }

    fn collect_remotes_packet(&self, mut packets: Vec<Packet<M>>) -> Vec<Packet<M>> {
        let mut msg = Message::new().unwrap();

        let mut sockets_to_poll: Vec<PollItem> =
            self.remotes_collector
                .iter()
                .map(|s| s.as_poll_item(POLLIN)).collect();

        zmq_poll(&mut sockets_to_poll, NONBLOCKING_POLL).unwrap();

        for (index_collector, socket) in sockets_to_poll.iter().enumerate() {
            while socket.is_readable() {
                if self.remotes_collector[index_collector].recv(&mut msg, 0).is_ok() {
                    trace!("Receive a packet from {}", index_collector);
                    if let Ok(p) = Packet::<M>::deserialize(&msg) {
                        packets.push(p);
                    } else {
                        error!("Receive a packet that can be deserialize");
                    }
                }
            }
        }

        packets
    }

    fn collect_local_packet(&self, mut packets: Vec<Packet<M>>) -> Vec<Packet<M>> {
        for packet in self.local_collector.try_iter() {
            packets.push(packet);
        }

        packets
    }
}