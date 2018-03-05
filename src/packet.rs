use agent_system::SystemId;
use agent::AgentId;

use std::cmp::Ordering;
use std::mem::transmute;
use std::error::Error;
use std::fmt;

pub enum Performative {
    Notify,
    NotifyAll,
    AskOne,
    AskAll,
    InReplyTo,
}

#[derive(Clone, Eq, Debug)]
pub struct Packet<P: Payload> {
    pub recipient: Recipient,
    pub sender: (SystemId, AgentId),
    pub priority: u8,
    pub occurred: u64,
    pub message: P,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Recipient {
    Agent { system_id: SystemId, agent_id: AgentId },
    Broadcast { system_id: Option<SystemId> },
}

/// Message exchange between agents from different systems
pub trait Payload: Eq + Clone {

    /// Convert a message read from socket into a message
    fn deserialize(bytes: &[u8]) -> Result<Self, ()>;

    /// Convert the message into str to be send to other system
    fn serialize(&self) -> Vec<u8>;

}

macro_rules! push_usize {
    ($u: expr, $buf: expr) => {
        unsafe {
            let s = transmute::<usize, [u8; 8]>($u);
            $buf.extend_from_slice(&s);
        }
    };
}

macro_rules! push_u64 {
    ($u: expr, $buf: expr) => {
        unsafe {
            let s = transmute::<u64, [u8; 8]>($u);
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

macro_rules! read_u64 {
    ($buf: expr) => {
        unsafe {
            let mut temp: [u8; 8] = Default::default();
            temp.copy_from_slice($buf);
            transmute::<[u8; 8], u64>(temp)
        }
    };
}

macro_rules! read_u8_from_cursor {
    ($cur: ident, $buf: ident) => {
        {
            $cur += 1;
            $buf[$cur-1]
        }
    };
}

macro_rules! read_usize_from_cursor {
    ($cur: expr, $buf: expr) => {
        {
            $cur += 8;
            read_usize!(&$buf[$cur - 8..$cur])
        }
    };
}

macro_rules! read_u64_from_cursor {
    ($cur: expr, $buf: expr) => {
        {
            $cur += 8;
            read_u64!(&$buf[$cur - 8..$cur])
        }
    };
}

#[derive(Debug)]
pub enum PacketSerdeError {
    PacketDeserializationErr,
    PacketSerializeErr,
    PayloadSerializeErr,
    PayloadDeserializationErr,
}

impl fmt::Display for PacketSerdeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            PacketSerdeError::PacketDeserializationErr => write!(f, "packet deserialization error"),
            PacketSerdeError::PacketSerializeErr => write!(f, "packet serialization error"),
            PacketSerdeError::PayloadSerializeErr => write!(f, "payload serialization error"),
            PacketSerdeError::PayloadDeserializationErr => write!(f, "packet deserialization error"),
        }
    }
}

impl Error for PacketSerdeError {
    fn description(&self) -> &str {
        //TODO: Improve this error message.
        "serde packet error"
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

pub const SEND_TO_AGENT: u8 = 0;
pub const BROADCAST_TO_SYSTEM: u8 = 1;
pub const BROADCAST_TO_ALL: u8 = 2;

impl <P: Payload>Packet<P> {

    pub fn deserialize(msg: &[u8]) -> Result<Self, PacketSerdeError> {
        let mut cursor = 0;

        let recipient = match read_u8_from_cursor!(cursor, msg) {
            SEND_TO_AGENT => {
                let system_id = read_u8_from_cursor!(cursor, msg);
                let agent_id = read_usize_from_cursor!(cursor, msg);
                Ok(Recipient::Agent{ system_id, agent_id })
            },
            BROADCAST_TO_SYSTEM => {
                let system_id = read_u8_from_cursor!(cursor, msg);
                Ok(Recipient::Broadcast{ system_id: Some(system_id) })
            },
            BROADCAST_TO_ALL => {
                Ok(Recipient::Broadcast{ system_id: None })
            },
            _ => Err(PacketSerdeError::PacketDeserializationErr),
        };

        recipient.and_then(|recipient| {
            let system_id = read_u8_from_cursor!(cursor, msg);
            let agent_id = read_usize_from_cursor!(cursor, msg);
            let priority = read_u8_from_cursor!(cursor, msg);
            let occurred = read_u64_from_cursor!(cursor, msg);

            let msg_size = read_usize_from_cursor!(cursor, msg);
            let still_to_read = msg_size + cursor;

            if  still_to_read == msg.len() {
                match Payload::deserialize(&msg[cursor..still_to_read]) {
                    Ok(message) => Ok(Self {
                        recipient,
                        sender: (system_id, agent_id),
                        priority,
                        occurred,
                        message,
                    }),
                    Err(_) => Err(PacketSerdeError::PayloadDeserializationErr),
                }
            } else {
                Err(PacketSerdeError::PacketDeserializationErr)
            }
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
        push_u64!(self.occurred, msg);

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

    const TEST_TIMESTAMP: u64 = 1520075517;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct Position {
        pos_x: u8,
        pos_y: u8,
    }

    impl Payload for Position {

        fn deserialize(bytes: &[u8]) -> Result<Self, ()> {
            if bytes.len() == 2 {
                Ok(Position {
                    pos_x: bytes[0],
                    pos_y: bytes[1],
                })
            }
            else {
                Err(())
            }
        }

        fn serialize(&self) -> Vec<u8> {
            [self.pos_x, self.pos_y].to_vec()
        }
    }

    #[test]
    fn it_should_serialize_a_broadcast_packet() {
        let packet = Packet {
            sender: (1, 2),
            recipient: Recipient::Broadcast{ system_id: None },
            priority: 3,
            occurred: TEST_TIMESTAMP,
            message: Position{ pos_x: 2, pos_y: 1},
        };

        let packet_expected = [
            BROADCAST_TO_ALL,                               // broadcast all code = 2
            0x01,                                           // sender system id
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // sender agent id
            0x03,                                           // priority
            0xFD, 0x82, 0x9A, 0x5A, 0x00, 0x00, 0x00, 0x00, // test occurred
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // message length
            0x02, 0x01,                                     // pos_x, pos_y
        ];

        assert_eq!(packet_expected , packet.serialize().as_slice());
    }

    #[test]
    fn it_should_serialize_a_packet_broadcast_to_a_another_system() {
        let packet = Packet {
            sender: (1, 2),
            recipient: Recipient::Broadcast{ system_id: Some(10) },
            priority: 3,
            occurred: TEST_TIMESTAMP,
            message: Position{ pos_x: 2, pos_y: 1},
        };

        let packet_expected = [
            BROADCAST_TO_SYSTEM,                            // broadcast to another system
            0x0A,                                           // recipient system id
            0x01,                                           // sender system id
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // sender agent id
            0x03,                                           // priority
            0xFD, 0x82, 0x9A, 0x5A, 0x00, 0x00, 0x00, 0x00, // test occurred
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // message length
            0x02, 0x01,                                     // pos_x, pos_y
        ];

        assert_eq!(packet_expected , packet.serialize().as_slice());
    }

    #[test]
    fn it_should_serialize_a_packet_addressed_to_another_agent() {
        let packet = Packet {
            sender: (1, 2),
            recipient: Recipient::Agent{ system_id: 1, agent_id: 8 },
            priority: 3,
            occurred: TEST_TIMESTAMP,
            message: Position{ pos_x: 2, pos_y: 1},
        };

        let packet_expected = [
            SEND_TO_AGENT,                                  // agent code
            0x01,                                           // recipient system id
            0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // recipient agent id
            0x01,                                           // sender system id
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // sender agent id
            0x03,                                           // priority
            0xFD, 0x82, 0x9A, 0x5A, 0x00, 0x00, 0x00, 0x00, // test occurred
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // message length
            0x02, 0x01,                                     // pos_x, pos_y
        ];

        assert_eq!(&packet_expected[..], packet.serialize().as_slice());
    }

    #[test]
    fn it_should_deserialize_a_broadcast_packet() {
        let packet_to_deserialize = [
            BROADCAST_TO_ALL,                               // broadcast to all
            0x01,                                           // sender system id
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // sender agent id
            0x03,                                           // priority
            0xFD, 0x82, 0x9A, 0x5A, 0x00, 0x00, 0x00, 0x00, // test occurred
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // message length
            0x02, 0x01,                                     // pos_x, pos_y
        ];

        let packet_expected = Packet {
            sender: (1, 2),
            recipient: Recipient::Broadcast{ system_id: None },
            priority: 3,
            occurred: TEST_TIMESTAMP,
            message: Position{ pos_x: 2, pos_y: 1},
        };

        let packet_deserialized = Packet::<Position>::deserialize(&packet_to_deserialize);
        assert!(packet_deserialized.is_ok());
        assert_eq!(packet_expected, packet_deserialized.unwrap());
    }

    #[test]
    fn it_should_deserialize_a_broadcast_to_another_system_packet() {
        let packet_to_deserialize = [
            BROADCAST_TO_SYSTEM,                            // broadcast to a system
            0x01,                                           // recipient system id
            0x01,                                           // sender system id
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // sender agent id
            0x03,                                           // priority
            0xFD, 0x82, 0x9A, 0x5A, 0x00, 0x00, 0x00, 0x00, // test occurred
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // message length
            0x02, 0x01,                                     // pos_x, pos_y
        ];

        let packet_expected = Packet {
            sender: (1, 2),
            recipient: Recipient::Broadcast{ system_id: Some(1) },
            priority: 3,
            occurred: TEST_TIMESTAMP,
            message: Position{ pos_x: 2, pos_y: 1},
        };

        let packet_deserialized = Packet::<Position>::deserialize(&packet_to_deserialize);
        assert!(packet_deserialized.is_ok());
        assert_eq!(packet_expected, packet_deserialized.unwrap());
    }

    #[test]
    fn it_should_deserialize_packet_addressed_to_another_agent() {
        let packet_to_deserialize = [
            SEND_TO_AGENT,                                  // packet for an agent
            0x01,                                           // recipient system id
            0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // recipient agent id
            0x01,                                           // sender system id
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // sender agent id
            0x03,                                           // priority
            0xFD, 0x82, 0x9A, 0x5A, 0x00, 0x00, 0x00, 0x00, // test occurred
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // message length
            0x02, 0x01,                                     // pos_x, pos_y
        ];

        let packet_expected = Packet {
            sender: (1, 2),
            recipient: Recipient::Agent{ system_id: 1 , agent_id: 8 },
            priority: 3,
            occurred: TEST_TIMESTAMP,
            message: Position{ pos_x: 2, pos_y: 1},
        };

        let packet_deserialized = Packet::<Position>::deserialize(&packet_to_deserialize);
        assert!(packet_deserialized.is_ok());
        assert_eq!(packet_expected, packet_deserialized.unwrap());
    }

    #[test]
    fn it_should_not_deserialize_packet_with_unknown_code() {
        let packet_to_deserialize = [
            42, // Unknown code
        ];

        let packet_deserialized = Packet::<Position>::deserialize(&packet_to_deserialize);
        assert!(packet_deserialized.is_err());
    }

    #[test]
    fn it_should_not_deserialize_packet_when_payload_is_not_deserializable() {
        let packet_with_incorrect_payload = [
            SEND_TO_AGENT,                                  // packet for an agent
            0x01,                                           // recipient system id
            0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // recipient agent id
            0x01,                                           // sender system id
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // sender agent id
            0x03,                                           // priority
            0xFD, 0x82, 0x9A, 0x5A, 0x00, 0x00, 0x00, 0x00, // test occurred
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // message length
            0x02,                                           // pos_x, [MISSING] the pos_y
        ];

        let packet_deserialized = Packet::<Position>::deserialize(&packet_with_incorrect_payload);
        assert!(packet_deserialized.is_err());
    }

    #[test]
    fn it_should_not_deserialize_packet_when_payload_is_to_small() {
        let packet_with_incorrect_payload_size = [
            SEND_TO_AGENT,                                   // packet for an agent
            0x01,                                            // recipient system id
            0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // recipient agent id
            0x01,                                            // sender system id
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // sender agent id
            0x03,                                            // priority
            0xFD, 0x82, 0x9A, 0x5A, 0x00, 0x00, 0x00, 0x00, // test occurred

            0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // [INCORRECT] message length
            0x02, 0x02,                                      // pos_x, the pos_y
        ];

        let packet_deserialized = Packet::<Position>::deserialize(&packet_with_incorrect_payload_size);
        assert!(packet_deserialized.is_err());
    }

    #[test]
    fn it_should_not_deserialize_packet_when_payload_is_to_big() {
        let packet_with_incorrect_payload_size = [
            SEND_TO_AGENT,                                   // agent infos recipient
            0x01,                                            // recipient system id
            0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // recipient agent id
            0x01,                                            // sender system id
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // sender agent id
            0x03,                                            // priority
            0xFD, 0x82, 0x9A, 0x5A, 0x00, 0x00, 0x00, 0x00, // test occurred

            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // [INCORRECT] message length
            0x02, 0x02,                                      // pos_x, the pos_y
        ];

        let packet_deserialized = Packet::<Position>::deserialize(&packet_with_incorrect_payload_size);
        assert!(packet_deserialized.is_err());
    }
}