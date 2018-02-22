use packet::Packet;
use packet::Payload;

pub type AgentId = usize;

pub trait Agent {
    type P: Payload;

    fn id(&self) -> AgentId;

    fn set_id(&mut self, id: AgentId);

    fn is_dead(&self) -> bool;

    fn handle_message(&mut self, packet: &Packet<Self::P>);

    fn update(&mut self) -> Option<Vec<Packet<Self::P>>>;
}