// Share between person system and taxi.
extern crate eden;

use eden::packet::Payload;

#[derive(Clone, Eq, PartialEq)]
pub enum ProtocolTaxi {
    AskForATaxi,
    AnswerTaxiAvailable,
    AnswerTaxiOk(usize),
}

impl Payload for ProtocolTaxi {
    fn deserialize(bytes: &[u8]) -> Result<Self, &str> {
        Ok(ProtocolTaxi::AskForATaxi)
    }

    fn serialize(&self) -> Vec<u8> {
        vec![]
    }
}

fn main() {}