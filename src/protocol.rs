/// Message exchange between agents from different systems
pub trait Protocol {
    type Message: Clone;

    /// Convert a message read from socket into a message
    fn serialize(&self, bytes: &[u8]) -> Self::Message;

    /// Convert the message into str to be send to other system
    fn deserialize(&self) -> &str;

}