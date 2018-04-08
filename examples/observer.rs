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
pub struct Observer {
    id: usize,
}

impl Agent for Observer {
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

    fn handle_message(&mut self, message: &Message<Self::C>) {
        match message.performative {
            Performative::Inform => {
                match message.content {
                    Protocol::Event(id) => {
                        println!("Receive the event: {}", id);
                    },
                };
            }
            _ => panic!("Should not receive this message"),
        }
    }

    fn act(&mut self) -> Option<Vec<Message<Self::C>>> {
        None
    }
}

pub struct ObserverFactory;

impl AgentFactory<Observer> for ObserverFactory {
    fn create(&self, agent_id: usize) -> Observer {
        Observer { id: agent_id }
    }
}

fn main() {
    let mut obs_sys: AgentSystem<Observer, Protocol>;
    let addr_system = SocketAddr::from(([127, 0, 0, 1], OBSERVER_SYSTEM_PORT));

    obs_sys = AgentSystem::new(
        OBSERVER_SYSTEM_ID,
        Box::new(ObserverFactory {}),
        addr_system,
    );

    obs_sys.add_remote_observer_system(
        SUBJECT_SYSTEM_ID,
        SocketAddr::from(([127, 0, 0, 1], SUBJECT_SYSTEM_PORT)),
    );

    obs_sys.spawn_agent();

    let mut resources = Resources::new();
    let mut dispatcher = DispatcherBuilder::new().add(obs_sys, "observer", &[]).build();

    loop {
        dispatcher.dispatch(&mut resources);
    }
}
