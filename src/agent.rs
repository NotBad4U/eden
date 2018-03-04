use agent_system::SystemId;
use packet::{Recipient, Packet, Payload};

pub type AgentId = usize;

pub struct Message<M> {
    pub recipient: Recipient,
    pub priority: u8,
    pub message: M,
}

impl <M: Payload>Message<M> {
    pub fn as_packet(self, sender: (SystemId, AgentId), occurred: u64) -> Packet<M> {
        Packet {
            sender,
            recipient: self.recipient,
            priority: self.priority,
            occurred,
            message: self.message,
        }
    }
}

pub trait Agent {
    type P: Payload;

    fn id(&self) -> AgentId;

    fn set_id(&mut self, id: AgentId);

    fn is_dead(&self) -> bool;

    fn handle_message(&mut self, packet: &Packet<Self::P>);

    fn update(&mut self) -> Option<Vec<Message<Self::P>>>;
}