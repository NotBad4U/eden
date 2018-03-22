#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

extern crate eden;
extern crate shred;
extern crate rand;
#[macro_use]
extern crate log;
extern crate env_logger;

mod protocol;

use protocol::ProtocolTaxi;
use eden::agent::{Agent, Message};
use eden::agent_system::AgentSystem;
use eden::agent_factory::AgentFactory;
use eden::packet::*;

use shred::{DispatcherBuilder, Resources};
use rand::{thread_rng, Rng};

use std::net::SocketAddr;

enum State {
    Idle,
    Get(usize),
    Carry(usize),
}

struct Taxi {
    id: usize,
    pos: (i32, i32),
    state: State,
}

impl Agent for Taxi {
    type P = ProtocolTaxi;

    fn id(&self) -> usize { self.id }

    fn set_id(&mut self, id: usize) { self.id = id }

    fn is_dead(&self) -> bool { false }

    fn handle_message(&mut self, packet: &Packet<Self::P>) {
        let sender_id = packet.sender.1;
        match packet.message {
            ProtocolTaxi::AskForATaxi => println!("The person {} ask for a taxi", sender_id),
            _ => eprintln!("Not my business"),
        };
    }

    fn act(&mut self) -> Option<Vec<Message<Self::P>>> {
        None
    }
}

struct TaxiFactory;

impl AgentFactory<Taxi> for TaxiFactory {

    fn create(&self, agent_id: usize) -> Taxi {
        let mut rng = thread_rng();

        Taxi {
            id: agent_id,
            pos: (rng.gen_range(0, 50), rng.gen_range(0, 50)),
            state: State::Idle,
        }
    }
}

const PERSON_SYSTEM_ID: u8 = 1;

fn main() {
    env_logger::init();

    let mut system: AgentSystem<Taxi, ProtocolTaxi>;
    let id_system = 2;
    let addr_system = SocketAddr::from(([127, 0, 0, 1], 8081));

    system = AgentSystem::new(id_system, Box::new(TaxiFactory), addr_system);
    system.add_remote_observer_system(
        (PERSON_SYSTEM_ID, SocketAddr::from(([127, 0, 0, 1], 8080)))
    );
    system.spawn_agent();

    let mut resources = Resources::new();
    let mut dispatcher = DispatcherBuilder::new()
        .add(system, "taxis", &[])
        .build();

    loop {
        dispatcher.dispatch(&mut resources);
    }
}
