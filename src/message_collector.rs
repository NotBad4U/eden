use zmq::{Socket, Context, SUB, PollItem, POLLIN, poll as zmq_poll, Message};

use packet::Packet;

use std::sync::mpsc::Receiver;
use std::net::SocketAddr;
use std::rc::Rc;

const BROADCAST_FILTER: &'static [u8] = b"BROADCAST";
const AGENT_FILTER: &'static str = "AGENT";
const AGENTS_FILTER: &'static str = "AGENTS";
const NONBLOCKING_POLL: i64 = 0;

pub struct Collector<M: Clone> {
    system_id: u8,
    zmq_ctx: Rc<Context>,
    local_collector: Receiver<Packet<M>>,
    remotes_collector: Vec<Socket>,
}

impl <M: Clone>Collector<M> {

    pub fn new(system_id: u8,
        zmq_ctx: Rc<Context>,
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
        //TODO: Manage errors
        let zmq_subscriber = self.zmq_ctx.socket(SUB).unwrap();
        zmq_subscriber.connect(&format!("tcp://{}", addr));

        zmq_subscriber.set_subscribe(format!("{}", self.system_id).as_bytes());
        zmq_subscriber.set_subscribe(BROADCAST_FILTER);
        zmq_subscriber.set_subscribe(format!("{} {}", self.system_id, AGENT_FILTER).as_bytes());
        zmq_subscriber.set_subscribe(format!("{} {}", self.system_id, AGENTS_FILTER).as_bytes());

        self.remotes_collector.push(zmq_subscriber);
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
            if socket.is_readable() {
                if self.remotes_collector[index_collector].recv(&mut msg, 0).is_ok() {
                    //TODO: add packet to packets buffer but first find a way to deser to <M>
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