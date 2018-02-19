use std::cmp::Ordering;

#[derive(Clone, Eq, PartialEq)]
pub enum Recipient {
    Agent { agent_id: usize },
    Broadcast,
}

/// Message exchange between agents from different systems
pub trait Payload: {
    type Message: Clone;

    /// Convert a message read from socket into a message
    fn serialize(&self, bytes: &[u8]) -> Self::Message;

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
pub struct Packet<M: Clone> {
    pub system_id: u8,
    pub priority: u8,
    pub sender_id: usize,
    pub recipient: Recipient,
    pub message: M,
}

impl <M: Eq + Clone>Ord for Packet<M> {
    fn cmp(&self, other: &Packet<M>) -> Ordering {
        self.priority.cmp(&other.priority)
    }
}

impl <M: Eq + Clone>PartialOrd for Packet<M> {
    fn partial_cmp(&self, other: &Packet<M>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl <M: Eq + Clone>PartialEq for Packet<M> {
    fn eq(&self, other: &Packet<M>) -> bool {
        self.recipient.eq(&other.recipient)
        && self.sender_id == other.sender_id
        && self.priority == other.priority
    }
}