use agent_system::SystemId;
use agent::AgentId;

use zmq::Message;

use std::cmp::Ordering;
use std::mem::transmute;


#[derive(Clone, Eq, PartialEq)]
pub enum Recipient {
    Agent { system_id: SystemId, agent_id: AgentId },
    Broadcast { system_id: Option<SystemId> },
}

/// Message exchange between agents from different systems
pub trait Payload: Eq + Clone {

    /// Convert a message read from socket into a message
    fn serialize(bytes: &[u8]) -> Result<Self, &str>;

    /// Convert the message into str to be send to other system
    fn deserialize(&self) -> &[u8];

}


#[derive(Clone, Eq)]
pub struct Packet<P: Payload> {
    pub priority: u8,
    pub sender: (SystemId, AgentId),
    pub recipient: Recipient,
    pub message: P,
}

macro_rules! push_usize {
    ($u: expr, $buf: expr) => {
        unsafe {
            let s = transmute::<usize, [u8; 8]>($u);
            $buf.extend_from_slice(&s);
        }
    };
}

const SEND_TO_AGENT: u8 = 0;
const BROADCAST_TO_AS_YSTEM: u8 = 1;
const BROADCAST_TO_ALL: u8 = 2;

impl <P: Payload>Packet<P> {

    pub fn serialize(&self, msg: &[u8]) -> Result<Self, &str> {
        let mut cursor = 0;

        let recipient = match msg[0] {
            SEND_TO_AGENT => {
                cursor += 2;
                Ok(Recipient::Agent{ system_id: msg[1], agent_id: 0 })
            },
            BROADCAST_TO_AS_YSTEM => {
                cursor += 2;
                Ok(Recipient::Broadcast{ system_id: Some(msg[1]) })
            },
            BROADCAST_TO_ALL => {
                cursor += 1;
                Ok(Recipient::Broadcast{ system_id: None })
            },
            _ => Err("Error during parsing"),
        };

        recipient.and_then(|recipient| {
            let priority = msg[cursor];
            let msg_size = 0;
            let message: P = Payload::serialize(&msg[cursor..msg_size]).unwrap(); //FIXME: a bit hard

            Ok(Self {
                priority,
                sender: (0, 0),
                recipient,
                message,
            })
        })
    }

    pub fn deserialize(& self) -> Vec<u8> {
        let mut msg: Vec<u8> = Vec::with_capacity(144);

        match self.recipient {
            Recipient::Agent { system_id, agent_id } => {
                msg.extend_from_slice(&[0, system_id]);
                push_usize!(agent_id, msg);
            },
            Recipient::Broadcast { ref system_id } => {
                if let &Some(system_id) = system_id {
                    msg.extend_from_slice(&[1, system_id]);
                } else {
                    msg.push(2);
                }
            }
        };

        msg.push(self.priority);

        let payload = self.message.deserialize();
        push_usize!(payload.len(), msg);
        msg.extend_from_slice(payload);

        msg
    }
}

impl <M: Payload>Ord for Packet<M> {
    fn cmp(&self, other: &Packet<M>) -> Ordering {
        self.priority.cmp(&other.priority)
    }
}

impl <M: Payload>PartialOrd for Packet<M> {
    fn partial_cmp(&self, other: &Packet<M>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl <M: Payload>PartialEq for Packet<M> {
    fn eq(&self, other: &Packet<M>) -> bool {
        self.recipient.eq(&other.recipient)
        && self.sender == other.sender
        && self.priority == other.priority
    }
}