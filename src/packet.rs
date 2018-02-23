use agent_system::SystemId;
use agent::AgentId;

use std::cmp::Ordering;
use std::mem::transmute;


#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Recipient {
    Agent { system_id: SystemId, agent_id: AgentId },
    Broadcast { system_id: Option<SystemId> },
}

/// Message exchange between agents from different systems
pub trait Payload: Eq + Clone {

    /// Convert a message read from socket into a message
    fn deserialize(bytes: &[u8]) -> Result<Self, &str>;

    /// Convert the message into str to be send to other system
    fn serialize(&self) -> Vec<u8>;

}


#[derive(Clone, Eq, Debug)]
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

macro_rules! read_usize {
    ($buf: expr) => {
        unsafe {
            let mut temp: [u8; 8] = Default::default();
            temp.copy_from_slice($buf);
            transmute::<[u8; 8], usize>(temp)
        }
    };
}

const SEND_TO_AGENT: u8 = 0;
const BROADCAST_TO_AS_YSTEM: u8 = 1;
const BROADCAST_TO_ALL: u8 = 2;

impl <P: Payload>Packet<P> {

    pub fn deserialize(msg: &[u8]) -> Result<Self, &str> {
        let mut cursor = 0;

        let recipient = match msg[0] {
            SEND_TO_AGENT => {
                cursor += 2; //FIXME: incorrect move
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
            //TODO: Make a macro to read and move the cursor
            let system_id = msg[cursor];
            cursor += 1;
            let agent_id = read_usize!(&msg[cursor..cursor+8]);
            cursor += 8;
            let priority = msg[cursor];
            cursor += 1;
            let msg_size = read_usize!(&msg[cursor..cursor+8]);
            cursor += 8;
            let message: P = Payload::deserialize(&msg[cursor..cursor+msg_size]).unwrap(); //FIXME: a bit hard

            Ok(Self {
                priority,
                sender: (system_id, agent_id),
                recipient,
                message,
            })
        })
    }

    pub fn serialize(& self) -> Vec<u8> {
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

        msg.push(self.sender.0);
        push_usize!(self.sender.1, msg);
        msg.push(self.priority);

        let payload = self.message.serialize();
        push_usize!(payload.len(), msg);
        msg.extend_from_slice(&payload);

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

#[cfg(test)]
mod test {

    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct Position {
        pos_x: u8,
        pos_y: u8,
    }

    impl Payload for Position {

        fn deserialize(bytes: &[u8]) -> Result<Self, &str> {
            if bytes.len() == 2 {
                Ok(Position {
                    pos_x: bytes[0],
                    pos_y: bytes[1],
                })
            }
            else {
                Err("serialization error")
            }
        }

        fn serialize(&self) -> Vec<u8> {
            [self.pos_x, self.pos_y].to_vec()
        }
    }

    #[test]
    fn it_should_serialize_a_packet() {
        let packet = Packet {
            sender: (1, 2),
            recipient: Recipient::Broadcast{ system_id: None },
            priority: 3,
            message: Position{ pos_x: 2, pos_y: 1},
        };

        let packet_expected = [
            BROADCAST_TO_ALL,                               // broadcast all code = 2
            0x01,                                           // agent id
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // system id
            0x03,                                           // priority
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // message length
            0x02, 0x01,                                     // pos_x, pos_y
        ];

        assert_eq!(packet_expected , packet.serialize().as_slice());
    }

    #[test]
    fn it_should_deserialize_a_packet() {
        let packet_to_deserialize = [
            BROADCAST_TO_ALL,                               // broadcast all code = 2
            0x01,                                           // agent id
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // system id
            0x03,                                           // priority
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // message length
            0x02, 0x01,                                     // pos_x, pos_y
        ];

        let packet_expected = Packet {
            sender: (1, 2),
            recipient: Recipient::Broadcast{ system_id: None },
            priority: 3,
            message: Position{ pos_x: 2, pos_y: 1},
        };

        let packet_deserialized = Packet::<Position>::deserialize(&packet_to_deserialize);
        assert!(packet_deserialized.is_ok());
        assert_eq!(packet_expected, packet_deserialized.unwrap());
    }
}