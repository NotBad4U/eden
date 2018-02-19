use zmq::{Socket, Context, PUB};

use packet::{Packet, Recipient, Payload};

use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::net::SocketAddr;
use std::sync::Arc;
use std::vec::Drain;

const NO_FLAGS: i32 = 0;
const BROADCAST_FILTER: &'static str = "BROADCAST";

pub struct Dispatcher<M: Payload> {
    system_id: u8,
    zmq_ctx: Arc<Context>,
    local_senders: HashMap<u8, Sender<Packet<M>>>,
    broadcast_publisher: Socket,
}

impl <M: Payload>Dispatcher<M> {

    pub fn new(system_id: u8, zmq_ctx: Arc<Context>, addr: SocketAddr) -> Self {
        Dispatcher {
            system_id,
            zmq_ctx,
            local_senders: HashMap::new(),
            broadcast_publisher: create_publisher_chan_for_broadcast(addr)
        }
    }

    pub fn add_local_sender(&mut self, sys_id: u8, sender: Sender<Packet<M>>) {
        self.local_senders.insert(sys_id, sender);
    }

    pub fn dispatch_packets(&self, packets: Drain<Packet<M>>) {
        for p in packets {
            if self.local_senders.contains_key(&p.system_id) {
                self.forward_message_to_local_sytem(p);
            }
            else /* remote system */ {
                self.forward_message_to_remote_sytem(p);
            }
        }
    }

    fn forward_message_to_local_sytem(&self, packet: Packet<M>) {
        match packet.recipient {
            Recipient::Agent{ agent_id } => {
                if let Some(sender) = self.local_senders.get(&packet.system_id) {
                    debug!("send a packet to agent {} in the local system {}", agent_id, packet.system_id);
                    sender.send(packet);
                }
            },
            Recipient::Broadcast => {
                for (_, sender) in self.local_senders.iter() {
                    debug!("broadcast a packet to all local observers systems");
                    sender.send(packet.clone());
                }
            },
        }
    }

    fn forward_message_to_remote_sytem(&self, packet: Packet<M>) {
        let key = match packet.recipient {
            Recipient::Agent{ agent_id } => {
                debug!("send packet to agent {} in the remote system {}", agent_id, packet.system_id);
                format!("AGENT {} {}", self.system_id, agent_id)
            },
            Recipient::Broadcast => {
                debug!("broadcast a packet to all remotes observers systems");
                format!("{}", BROADCAST_FILTER)
            },
        };
        self.broadcast_publisher.send(b"yolo", NO_FLAGS).expect("");
    }
}

fn create_publisher_chan_for_broadcast(addr: SocketAddr) -> Socket {
    //TODO: Manage errors
    let context = Context::new();
    let zmq_publisher = context.socket(PUB).unwrap();
    zmq_publisher.bind(&format!("tcp://{}", addr)).expect("failed binding publisher");
    info!("Remote publisher is ready to send message on {}", addr);

    zmq_publisher
}