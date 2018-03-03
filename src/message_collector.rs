use zmq::{Socket, Context, SUB, PollItem, POLLIN, poll as zmq_poll, Message};

use packet::{Packet, Payload, BROADCAST_TO_ALL, BROADCAST_TO_SYSTEM, SEND_TO_AGENT};

use std::sync::mpsc::Receiver;
use std::collections::VecDeque;
use std::collections::vec_deque::Drain;
use std::net::SocketAddr;
use std::sync::Arc;

const NONBLOCKING_POLL: i64 = 0;
const INBOX_CAPACITY: usize = 128;

pub struct Collector<M: Payload> {
    system_id: u8,
    zmq_ctx: Arc<Context>,
    local_collector: Receiver<Packet<M>>,
    remotes_collector: Vec<Socket>,
    inbox: VecDeque<Packet<M>>,
}

impl <M: Payload>Collector<M> {

    pub fn new(system_id: u8,
        zmq_ctx: Arc<Context>,
        local_collector: Receiver<Packet<M>>,
        inbox_capacity: Option<usize>,
    ) -> Self {
        Collector {
            system_id,
            zmq_ctx,
            local_collector,
            remotes_collector: Vec::new(),
            inbox: VecDeque::with_capacity(inbox_capacity.unwrap_or(INBOX_CAPACITY)),
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

    pub fn drain_inbox(&mut self) -> Option<Drain<Packet<M>>> {
        if self.inbox.len() > 0 {
            return Some(self.inbox.drain(..))
        }

        None
    }

    pub fn collect_packets(&mut self) {
        self.collect_remotes_packet();
        self.collect_local_packet();
    }

    fn collect_remotes_packet(&mut self) {
        let mut msg = Message::new().unwrap();

        let mut sockets_to_poll: Vec<PollItem> =
            self.remotes_collector
                .iter()
                .map(|s| s.as_poll_item(POLLIN)).collect();

        zmq_poll(&mut sockets_to_poll, NONBLOCKING_POLL).unwrap();

        for (index_collector, socket) in sockets_to_poll.iter().enumerate() {
            if socket.is_readable() {
                while self.remotes_collector[index_collector].recv(&mut msg, 0).is_ok() {
                    if self.inbox.len() < self.inbox.capacity() {
                        if let Ok(packet) = Packet::<M>::deserialize(&msg) {
                            self.inbox.push_back(packet);
                        } else {
                            trace!("Receive a packet that can be deserialize");
                        }
                    } else {
                        trace!("Can't receive more messages, the inbox is filled");
                        break;
                    }
                }
            }
        }
    }

    fn collect_local_packet(&mut self) {
        for packet in self.local_collector.try_iter() {
            self.inbox.push_back(packet);

            if self.inbox.capacity() == 0 {
                break;
            }
        }
    }
}