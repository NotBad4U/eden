#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

// Shared between person system and taxi.
extern crate eden;

use eden::packet::Payload;

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum ProtocolTaxi {
    AskForATaxi,
    AnswerTaxiAvailable,
    AnswerTaxiOk(usize),
}

impl Payload for ProtocolTaxi {
    fn deserialize(bytes: &[u8]) -> Result<Self, ()> {
        Ok(ProtocolTaxi::AskForATaxi)
    }

    fn serialize(&self) -> Vec<u8> {
        vec![]
    }
}

fn main() {}