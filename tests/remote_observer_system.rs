extern crate eden;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate shred;

use std::{thread, net::SocketAddr, sync::mpsc::{channel, Sender}};

use eden::{agent::*, agent_factory::*, agent_system::*, message::*};
use shred::{DispatcherBuilder, Resources};

pub const MAGIC_EVENT: u8 = 123;

pub const SUBJECT_SYSTEM_ID: u8 = 0;
pub const OBSERVER_SYSTEM_ID: u8 = 1;

pub const SUBJECT_SYSTEM_PORT: u16 = 8000;
pub const OBSERVER_SYSTEM_PORT: u16 = 8001;

/*
Flow diagram for the test below.
NOTE: That show how the main process wait for the end of the execution of the subject and observer systems.
Observer system only stop when he receive an event from the subject system.

                  send msg               notify to stop
     Subject ---------------------------------*---------X
            /       |               ack       |
Master ------------------------------*-----------X
            \       |                |
    Observer -------*-----[-----]----------X
                msg recv   assert  notify
*/
#[test]
fn remote_observer_should_receive_messages_from_subscribed_remote_system() {
    let (sender_master, receiver_master) = channel();

    thread::spawn(move || {
        let mut sub_sys: AgentSystem<Subject, Protocol>;
        let addr_system = SocketAddr::from(([127, 0, 0, 1], SUBJECT_SYSTEM_PORT));

        sub_sys = AgentSystem::new(SUBJECT_SYSTEM_ID, Box::new(SubjectFactory {}), addr_system);
        sub_sys.spawn_agent();

        let mut resources = Resources::new();
        let mut dispatcher = DispatcherBuilder::new()
            .add(sub_sys, "subject", &[])
            .build();

        'main: loop {
            dispatcher.dispatch(&mut resources);

            if let Ok(_) = receiver_master.try_recv() {
                break 'main;
            }
        }
    });

    let obs = thread::spawn(move || {
        let (sender, receiver) = channel();

        let mut obs_sys: AgentSystem<Observer, Protocol>;
        let addr_system = SocketAddr::from(([127, 0, 0, 1], OBSERVER_SYSTEM_PORT));

        obs_sys = AgentSystem::new(
            OBSERVER_SYSTEM_ID,
            Box::new(ObserverFactory { sender }),
            addr_system,
        );

        obs_sys.add_remote_observer_system(
            SUBJECT_SYSTEM_ID,
            SocketAddr::from(([127, 0, 0, 1], SUBJECT_SYSTEM_PORT)),
        );

        obs_sys.spawn_agent();

        let mut resources = Resources::new();
        let mut dispatcher = DispatcherBuilder::new()
            .add(obs_sys, "observer", &[])
            .build();

        'main: loop {
            dispatcher.dispatch(&mut resources);

            if let Ok(_) = receiver.try_recv() {
                break 'main;
            }
        }
    });

    match obs.join() {
        Ok(_) => sender_master
            .send(())
            .expect("Should send the message to Subject system"),
        Err(e) => panic!(e),
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Protocol {
    Event(u8),
}

impl Content for Protocol {}

#[derive(Serialize, Deserialize, Clone)]
pub struct Subject {
    id: usize,
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
                Protocol::Event(MAGIC_EVENT),
            ),
        ])
    }
}

pub struct SubjectFactory;

impl AgentFactory<Subject> for SubjectFactory {
    fn create(&self, agent_id: usize) -> Subject {
        Subject { id: agent_id }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Observer {
    id: usize,
    // Set at Option for Deserialize which use Default::default for the field attribute: skip
    #[serde(skip)]
    sender: Option<Sender<()>>,
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
                    Protocol::Event(n) => {
                        // Tell the thread controller to stop the system
                        assert_eq!(MAGIC_EVENT, n);
                        if let Some(ref sender) = self.sender {
                            sender.send(()).expect("Should send the message to master");
                        }
                    }
                };
            }
            _ => panic!("Should not receive this message"),
        }
    }

    fn act(&mut self) -> Option<Vec<Message<Self::C>>> {
        None
    }
}

pub struct ObserverFactory {
    sender: Sender<()>,
}

impl AgentFactory<Observer> for ObserverFactory {
    fn create(&self, agent_id: usize) -> Observer {
        Observer {
            id: agent_id,
            sender: Some(self.sender.clone()),
        }
    }
}
