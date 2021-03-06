extern crate eden;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate shred;

use std::{thread, net::SocketAddr, sync::mpsc::{channel, Sender}};

use eden::{agent::*, agent_factory::*, agent_system::*, message::*};
use shred::{DispatcherBuilder, Resources, Dispatcher};

pub const MAGIC_EVENT: u8 = 123;

pub const SUBJECT_SYSTEM_ID: u8 = 0;
pub const SUBJECT_SYSTEM_PORT: u16 = 4000;

#[test]
fn all_remote_observer_should_receive_for_broadcasted_message() {
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

    let obs1 = thread::spawn(move || {
        let (sender, receiver) = channel();
        let mut dispatcher = new_observer_with_two_agents(
            1,
            SocketAddr::from(([127, 0, 0, 1], 4001)),
            sender,
        );
        let mut resources = Resources::new();
        let mut nb_msg = 0;

        'main: loop {
            dispatcher.dispatch(&mut resources);

            if let Ok(_) = receiver.try_recv() {
                nb_msg+= 1;
            }

            if nb_msg == 2 {
                break 'main;
            }
        }
    });

    let obs2 = thread::spawn(move || {
        let (sender, receiver) = channel();
        let mut dispatcher = new_observer_with_two_agents(
            1,
            SocketAddr::from(([127, 0, 0, 1], 4002)),
            sender,
        );
        let mut resources = Resources::new();
        let mut nb_msg = 0;

        'main: loop {
            dispatcher.dispatch(&mut resources);

            if let Ok(_) = receiver.try_recv() {
                nb_msg+= 1;
            }

            if nb_msg == 2 {
                break 'main;
            }
        }
    });

    obs2.join().expect("Should expect the observer 2");

    // When the last observer has finish then we stop the subject.
    match obs1.join() {
        Ok(_) => sender_master
            .send(())
            .expect("Should send the message to Subject system"),
        Err(e) => panic!(e),
    }
}


fn new_observer_with_two_agents<'a, 'b>(id: u8, addr_system: SocketAddr, sender: Sender<()>) -> Dispatcher<'a, 'b> {
    let mut obs_sys: AgentSystem<Observer, Protocol>;

    obs_sys = AgentSystem::new(
        id,
        Box::new(ObserverFactory { sender }),
        addr_system,
    );

    obs_sys.add_remote_observer_system(
        SUBJECT_SYSTEM_ID,
        SocketAddr::from(([127, 0, 0, 1], SUBJECT_SYSTEM_PORT)),
    );

    obs_sys.spawn_agent();
    obs_sys.spawn_agent();

    DispatcherBuilder::new()
        .add(obs_sys, "observer", &[])
        .build()
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
                Recipient::Broadcast { system_id: None },
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
    already_receive: bool,
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
                            if !self.already_receive {
                                self.already_receive = true;
                                sender.send(()).expect("Should send the message to master");
                            }
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
            already_receive: false,
        }
    }
}
