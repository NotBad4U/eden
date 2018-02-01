#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use slab::Slab;

use agent::Agent;
use packet::{Packet, Recipient};

pub struct Inbox<M: Eq + Clone> {
    inboxes: Vec<Packet<M>>,
}

impl <M: Eq + Clone>Inbox<M> {

    pub fn new() -> Self {
        Inbox {
            inboxes: Vec::new(),
        }
    }

    pub fn process_packets<A: Agent>(&self, agent: &mut Slab<A>) {

    }
}