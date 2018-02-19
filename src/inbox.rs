#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use slab::Slab;

use agent::Agent;
use packet::{Packet, Recipient, Payload};

pub struct Inbox<M: Payload> {
    inboxes: Vec<Packet<M>>,
}

impl <M: Payload>Inbox<M> {

    pub fn new() -> Self {
        Inbox {
            inboxes: Vec::new(),
        }
    }

    pub fn process_packets<A: Agent>(&self, agent: &mut Slab<A>) {

    }
}