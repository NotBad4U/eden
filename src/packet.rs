use std::cmp::Ordering;

#[derive(Clone, Eq, PartialEq)]
pub enum Recipient {
    Agent { agent_id: usize },
    Broadcast,
}

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