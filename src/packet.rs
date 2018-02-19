use std::cmp::Ordering;

#[derive(Clone, Eq, PartialEq)]
pub enum Recipient {
    Agent { agent_id: usize },
    Broadcast,
}

/// Message exchange between agents from different systems
pub trait Payload: Eq + Clone {

    /// Convert a message read from socket into a message
    fn serialize(&self, bytes: &[u8]) -> Self;

    /// Convert the message into str to be send to other system
    fn deserialize(&self) -> &str;

}

/// Packets send from the network take this layout.
///```
/// +---------------------+--------------+----------------+---------+
/// | 1bit                | 1bit         | 8bits          | unknow  |
/// +---------------------+--------------+----------------+---------+
/// | broadcast/system id | agents/agent | agent id/empty | payload |
/// +---------------------+--------------+----------------+---------+
///```
#[derive(Clone, Eq)]
pub struct Packet<P: Payload> {
    pub system_id: u8,
    pub priority: u8,
    pub sender_id: usize,
    pub recipient: Recipient,
    pub message: P,
}

impl <P: Payload>Packet<P> {

    pub fn serialize(&self, bytes: &[u8]) {

    }

    pub fn deserialize(&self) -> &str {
        ""
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
        && self.sender_id == other.sender_id
        && self.priority == other.priority
    }
}