use packet::Packet;

pub trait Agent {
    fn id(&self) -> usize;

    fn set_id(&mut self, id: usize);

    fn is_dead(&self) -> bool;

    fn handle_message<M: Clone>(&mut self, packet: &Packet<M>);

    fn update<M: Clone>(&mut self) -> Option<Vec<Packet<M>>>;
}