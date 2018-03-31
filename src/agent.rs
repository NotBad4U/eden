use serde::{Serialize, de::DeserializeOwned};

use message::*;

pub type AgentId = usize;

pub trait Agent {

    type Content: Serialize + DeserializeOwned + Clone + Eq;

    fn id(&self) -> AgentId;

    fn set_id(&mut self, id: AgentId);

    fn is_dead(&self) -> bool;

    fn handle_message(&mut self, message: &Message<Self::Content>);

    fn act(&mut self) -> Option<Vec<Message<Self::Content>>>;
}