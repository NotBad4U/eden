use std::cmp::Ordering;

#[derive(Clone, Eq, Hash)]
pub struct Packet<M> {
    pub priority: u8,
    pub type_agent: u8,
    pub recipient_id: usize,
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
        self.type_agent == other.type_agent
        && self.type_agent == other.type_agent
    }
}