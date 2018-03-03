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

enum StatePerson {
    WaitingTaxi(usize),
    WaitingAnswerTaxi,
    AskingTaxi,
    Working,
}

struct Person {
    id: usize,
    pos: (i32, i32),
    name: String,
    state: StatePerson,
}

impl Agent for Person {
    type P = ProtocolTaxi;

    fn id(&self) -> usize { self.id }

    fn set_id(&mut self, id: usize) { self.id = id }

    fn is_dead(&self) -> bool { false }

    fn handle_message(&mut self, packet: &Packet<Self::P>) {
        println!("I'm the person {} and I receive the packet {:?}", self.id(), packet);
    }

    fn update(&mut self) -> Option<Vec<Message<Self::P>>> {
        Some(vec![
            Message {
                priority: 1,
                recipient: Recipient::Broadcast{ system_id: Some(TAXI_SYSTEM_ID) },
                message: ProtocolTaxi::AskForATaxi,
            }
        ])
    }
}

struct PersonFactory;

impl AgentFactory<Person> for PersonFactory {
    fn create(&self, agent_id: usize) -> Person {
        let mut rng = thread_rng();

        Person {
            id: agent_id,
            pos: (rng.gen_range(0, 50), rng.gen_range(0, 50)),
            name: "Carl".to_string(),
            state: StatePerson::Working,
        }
    }
}

const PERSON_SYSTEM_ID: u8 = 1;
const TAXI_SYSTEM_ID: u8 = 2;

fn main() {
    env_logger::init();

    let mut system: AgentSystem<Person, ProtocolTaxi>;
    let id_system = PERSON_SYSTEM_ID;
    let addr_system = SocketAddr::from(([127, 0, 0, 1], 8080));

    system = AgentSystem::new(id_system, Box::new(PersonFactory), addr_system);
    system.add_remote_observer_system(
        (TAXI_SYSTEM_ID, SocketAddr::from(([127, 0, 0, 1], 8081)))
    );

    system.spawn_agent();

    let mut resources = Resources::new();
    let mut dispatcher = DispatcherBuilder::new()
        .add(system, "persons", &[])
        .build();

    loop {
        dispatcher.dispatch(&mut resources);
    }
}
