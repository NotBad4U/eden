use std::cmp::Ordering;

#[derive(Clone, Eq, PartialEq)]
pub struct Recipient {
    pub system_id: u8,
    pub agent_id: usize,
}

#[derive(Clone, Eq)]
pub struct Packet<M> {
    pub priority: u8,
    pub type_agent: u8,
    pub recipient: Recipient,
    pub message: M,
}

impl <M: Eq>Ord for Packet<M> {
    fn cmp(&self, other: &Packet<M>) -> Ordering {
        self.priority.cmp(&other.priority)
    }
}

impl <M: Eq>PartialOrd for Packet<M> {
    fn partial_cmp(&self, other: &Packet<M>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl <M: Eq>PartialEq for Packet<M> {
    fn eq(&self, other: &Packet<M>) -> bool {
        self.recipient.system_id == other.recipient.system_id
        && self.recipient.agent_id == other.recipient.agent_id
        && self.type_agent == other.type_agent
        && self.priority == other.priority
    }
}