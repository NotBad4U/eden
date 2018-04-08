extern crate eden;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate shred;

use std::net::SocketAddr;

use eden::{agent::*, agent_factory::*, agent_system::*, message::*};
use shred::{DispatcherBuilder, Resources};

//TODO: Find a way to share this const values
pub const SUBJECT_SYSTEM_ID: u8 = 0;
pub const OBSERVER_SYSTEM_ID: u8 = 1;

pub const SUBJECT_SYSTEM_PORT: u16 = 8000;
pub const OBSERVER_SYSTEM_PORT: u16 = 8001;

#[derive(Serialize, Deserialize, Clone)]
pub enum Protocol {
    Event(u8),
}

impl Content for Protocol {}

#[derive(Serialize, Deserialize, Clone)]
pub struct Subject {
    id: usize,
    event: u8,
}

impl Agent for Subject {
    type C = Protocol;

    fn id(&self) -> usize {
        self.id
    }

    fn set_id(&mut self, id: usize) {
        self.id = id
    }

    fn is_dead(&self) -> bool {
        false
    }

    fn handle_message(&mut self, _message: &Message<Self::C>) {
        unimplemented!();
    }

    fn act(&mut self) -> Option<Vec<Message<Self::C>>> {
        self.event = (self.event + 1) % 255;

        Some(vec![
            Message::new(
                Performative::Inform,
                Recipient::Agent {
                    agent_id: 0,
                    system_id: OBSERVER_SYSTEM_ID,
                },
                0,
                1,
                None,
                None,
                None,
                None,
                Protocol::Event(self.event),
            ),
        ])
    }
}

pub struct SubjectFactory;

impl AgentFactory<Subject> for SubjectFactory {
    fn create(&self, agent_id: usize) -> Subject {
        Subject {
            id: agent_id,
            event: 0,
        }
    }
}

fn main() {
    let mut sub_sys: AgentSystem<Subject, Protocol>;
    let addr_system = SocketAddr::from(([127, 0, 0, 1], SUBJECT_SYSTEM_PORT));

    sub_sys = AgentSystem::new(SUBJECT_SYSTEM_ID, Box::new(SubjectFactory {}), addr_system);
    sub_sys.spawn_agent();

    let mut resources = Resources::new();
    let mut dispatcher = DispatcherBuilder::new()
        .add(sub_sys, "subject", &[])
        .build();

    loop {
        dispatcher.dispatch(&mut resources);
    }
}
