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
    fn serialize(&self, bytes: &[u8]) -> Self {
        ProtocolTaxi::AskForATaxi
    }

    fn deserialize(&self) -> &str {
        ""
    }
}

fn main() {}