// Share between person system and taxi.

#[derive(Clone, Eq, PartialEq)]
pub enum ProtocolTaxi {
    AskForATaxi,
    AnswerTaxiAvailable,
    AnswerTaxiOk(usize),
}

fn main() {}