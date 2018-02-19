use packet::Packet;
use packet::Payload;

pub trait Agent {
    type P: Payload;

    fn id(&self) -> usize;

    fn set_id(&mut self, id: usize);

    fn is_dead(&self) -> bool;

    fn handle_message(&mut self, packet: &Packet<Self::P>);

    fn update(&mut self) -> Option<Vec<Packet<Self::P>>>;
}