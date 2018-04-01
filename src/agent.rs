use message::*;

pub type AgentId = usize;

pub trait Agent {

    type C: Content;

    fn id(&self) -> AgentId;

    fn set_id(&mut self, id: AgentId);

    fn is_dead(&self) -> bool;

    fn handle_message(&mut self, message: &Message<Self::C>);

    fn act(&mut self) -> Option<Vec<Message<Self::C>>>;
}