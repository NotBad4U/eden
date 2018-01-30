use packet::Packet;

pub trait Agent {
    fn id(&self) -> usize;

    fn set_id(&mut self, id: usize);

    fn is_dead(&self) -> bool;

    fn handle_message(&mut self, content: i32);

    fn update<M>(&mut self) -> Option<Vec<Packet<M>>>;
}