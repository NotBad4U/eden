use slab::Slab;
use shred::System;

use agent::Agent;
use agent_factory::AgentFactory;
use packet::{Packet, Recipient};
use networking::Session;

use std::sync::mpsc::{channel, Sender, Receiver};
use std::collections::HashMap;
use std::net::SocketAddr;

pub enum RemoteSystem<M: Clone> {
    Local {
        id: u8,
        channel_sender: Sender<Packet<M>>
    },
    Remote {
        id: u8,
        session: Session,
    }
}

impl <M: Clone>RemoteSystem<M> {
    fn dispatch(&mut self, packet: Packet<M>) {
        match self {
            &mut RemoteSystem::Local{ref id, ref channel_sender} => {
                channel_sender.send(packet.clone()).expect("send packet to other system");
            },
            &mut RemoteSystem::Remote{ref id, ref mut session} => {
                session.try_send();
            },
        }
    }
}

pub struct AgentSystem<A: Agent<P=M>, M: Eq + Clone> {
    id: u8, 
    agents: Slab<A>,
    inboxes: Vec<Packet<M>>,
    chan: (Sender<Packet<M>>, Receiver<Packet<M>>),
    views: HashMap<u8, RemoteSystem<M>>,
    factory: Box<AgentFactory<A> + Send>,
}

impl <A: Agent<P=M>, M: Eq + Clone>AgentSystem<A, M> {
    pub fn new(id: u8, factory: Box<AgentFactory<A> + Send>) -> Self {
        AgentSystem {
            id,
            agents: Slab::new(),
            inboxes: Vec::new(),
            views: HashMap::new(),
            chan: channel(),
            factory,
        }
    }

    pub fn spawn_agent(&mut self) {
        let entry_agent = self.agents.vacant_entry();
        let agent = self.factory.create(entry_agent.key());
        entry_agent.insert(agent);
    }

    pub fn spawn_swarm(&mut self, count: usize) {
        for _ in 0..count {
            self.spawn_agent();
        }
    }

    pub fn process_agent(&mut self) {
        for (_, agent) in self.agents.iter_mut() {
            if let Some(mut messages) = agent.update() {
                self.inboxes.append(&mut messages);
            }
        }
        self.inboxes.sort();
    }

    pub fn process_messages(&mut self) {
        for packet in self.inboxes.drain(..) {
            if packet.system_id == self.id {
                // Agent(s) on the same system.
                match packet.recipient {
                    Recipient::Agent{ agent_id } => {
                        let agent = self.agents.get_mut(agent_id);

                        if let Some(agent) = agent {
                            agent.handle_message(&packet);
                        }
                    },
                    Recipient::Broadcast => {
                        for (_, agent) in self.agents.iter_mut() {
                            if agent.id() != packet.sender_id {
                                agent.handle_message(&packet);
                            }
                        }
                    },
                }
            }
            else {
                // Agent(s) on another system.
                match packet.recipient {
                    Recipient::Broadcast => {
                        for (_, view) in self.views.iter_mut() {
                            view.dispatch(packet.clone());
                        }
                    },
                    _ => {
                        let system_view = self.views.get_mut(&packet.system_id);

                        if let Some(view) = system_view {
                            view.dispatch(packet);
                        }
                    }
                }
            }
        }
    }

    pub fn get_message_from_other_system(&mut self) {
        for packet in self.chan.1.try_iter() {
            self.inboxes.push(packet);
        }
        self.inboxes.sort();
    }

    pub fn get_sender(&self) -> Sender<Packet<M>> {
        self.chan.0.clone()
    }

    pub fn add_local_system(&mut self, id: u8, channel_sender: Sender<Packet<M>>) {
        let local_system = RemoteSystem::Local {
            id,
            channel_sender,
        };
        self.views.insert(id, local_system);
    }

    pub fn add_remote_system(&mut self, id: u8, addr: SocketAddr) {
        let session = Session::new(addr);
        let remote_system = RemoteSystem::Remote {
            id,
            session,
        };
        self.views.insert(id, remote_system);
    }

    pub fn get_nb_message_inbox(&self) -> usize {
        self.inboxes.len()
    }

    pub fn id(&self) -> u8 {
        self.id
    }

    pub fn get_nb_agents(&self) -> usize {
        self.agents.len()
    }
}

impl<'a, A: Agent<P=M>, M: Eq + Clone> System<'a> for AgentSystem<A, M> {
    type SystemData = ();

    fn run(&mut self, _: Self::SystemData) {
        self.process_agent();
        self.get_message_from_other_system();
        self.process_messages();
    }
}

#[cfg(test)]
mod test_sytem {

    use super::*;
    use shred::{DispatcherBuilder, Resources};

    struct Person {
        id: usize,
    }

    #[derive(Clone, Eq, PartialEq)]
    enum Protocol{}

    impl Agent for Person {
        type P = Protocol;

        fn id(&self) -> usize { self.id }

        fn set_id(&mut self, id: usize) { self.id = id }

        fn is_dead(&self) -> bool { false }

        fn handle_message(&mut self, _: &Packet<Self::P>) {}

        fn update(&mut self) -> Option<Vec<Packet<Self::P>>> {
            None
        }
    }

    struct PersonFactory;

    impl AgentFactory<Person> for PersonFactory {
        fn create(&self, agent_id: usize) -> Person {
            Person {
                id: agent_id,
            }
        }
    }


    #[test]
    fn it_should_spawn_an_agent() {
        let mut pers_sys: AgentSystem<Person, Protocol>;
        pers_sys = AgentSystem::new(0, Box::new(PersonFactory));

        pers_sys.spawn_agent();
        pers_sys.spawn_agent();

        assert_eq!(2, pers_sys.get_nb_agents());
    }

    #[test]
    fn it_should_spawn_a_swarm_of_agents() {
        let nb_agent_to_spawn = 10;
        let mut pers_sys: AgentSystem<Person, Protocol>;
        pers_sys = AgentSystem::new(0, Box::new(PersonFactory));

        pers_sys.spawn_swarm(nb_agent_to_spawn);

        assert_eq!(nb_agent_to_spawn, pers_sys.get_nb_agents());
    }

    struct AgentTestMsg {
        id: usize,
        id_other_agent: usize,
    }

    #[derive(Clone, Eq, PartialEq)]
    enum ProtocolGreeting {
        Greeting(usize),
    }

    impl Agent for AgentTestMsg {
        type P = ProtocolGreeting;

        fn id(&self) -> usize { self.id }

        fn set_id(&mut self, id: usize) { self.id = id }

        fn is_dead(&self) -> bool { false }

        fn handle_message(&mut self, packet: &Packet<Self::P>) {
            match packet.message {
                ProtocolGreeting::Greeting(id_agent) => { 
                    self.id_other_agent = id_agent
                },
            }
        }

        fn update(&mut self) -> Option<Vec<Packet<Self::P>>> {
            let my_id = self.id();
            Some(vec! [
                Packet {
                    system_id: 0,
                    priority: 1,
                    sender_id: my_id,
                    recipient: Recipient::Agent{ agent_id: 1 - my_id },
                    message: ProtocolGreeting::Greeting(my_id),
                }
            ])
        }
    }

    struct AgentTestMsgFactory;

    impl AgentFactory<AgentTestMsg> for AgentTestMsgFactory {
        fn create(&self, agent_id: usize) -> AgentTestMsg {
            AgentTestMsg {
                id: agent_id,
                id_other_agent: 0,
            }
        }
    }


    #[test]
    fn it_should_dispatch_message_between_same_agent() {
        let mut system: AgentSystem<AgentTestMsg, ProtocolGreeting>;
        system = AgentSystem::new(0, Box::new(AgentTestMsgFactory));
        system.spawn_swarm(2);

        let mut resources = Resources::new();
        let mut dispatcher = DispatcherBuilder::new()
            .add(system, "test", &[])
            .build();

        dispatcher.dispatch(&mut resources);
    }

    struct AgentTestMsgBroadcast {
        id: usize,
    }

    impl Agent for AgentTestMsgBroadcast {
        type P = ProtocolGreeting;

        fn id(&self) -> usize { self.id }

        fn set_id(&mut self, id: usize) { self.id = id }

        fn is_dead(&self) -> bool { false }

        fn handle_message(&mut self, packet: &Packet<Self::P>) {
            match packet.message {
                ProtocolGreeting::Greeting(_) => { 
                    // println!("I'm {} and I got Hello from {}", self.id(), id_agent);
                },
            }
        }

        fn update(&mut self) -> Option<Vec<Packet<Self::P>>> {
            let my_id = self.id();
            Some(vec! [
                Packet {
                    system_id: 0,
                    priority: 1,
                    sender_id: my_id,
                    recipient: Recipient::Broadcast,
                    message: ProtocolGreeting::Greeting(my_id),
                }
            ])
        }
    }

    struct GentTestMsgBroadcastFactory;

    impl AgentFactory<AgentTestMsgBroadcast> for GentTestMsgBroadcastFactory {
        fn create(&self, agent_id: usize) -> AgentTestMsgBroadcast {
            AgentTestMsgBroadcast {
                id: agent_id,
            }
        }
    }

    #[test]
    fn it_should_broadcast_message_between_same_agent() {
        let mut system: AgentSystem<AgentTestMsgBroadcast, ProtocolGreeting>;
        system = AgentSystem::new(0, Box::new(GentTestMsgBroadcastFactory));
        system.spawn_swarm(10);

        let mut resources = Resources::new();
        let mut dispatcher = DispatcherBuilder::new()
            .add(system, "test", &[])
            .build();

        dispatcher.dispatch(&mut resources);
    }

    struct AgentTestMsgBetweenSystem {
        id: usize,
        pos: (u8, u8),
        id_other_sytem: u8,
    }

    #[derive(Clone, Eq, PartialEq)]
    enum ProtocolPos {
        Position(u8, u8),
    }

    impl Agent for AgentTestMsgBetweenSystem {
        type P = ProtocolPos;

        fn id(&self) -> usize { self.id }

        fn set_id(&mut self, id: usize) { self.id = id }

        fn is_dead(&self) -> bool { false }

        fn handle_message(&mut self, packet: &Packet<Self::P>) {
            match packet.message {
                ProtocolPos::Position(x, y) => {
                    println!("The other agent is at the position x={} y={}", x, y);
                },
            }
        }

        fn update(&mut self) -> Option<Vec<Packet<Self::P>>> {
            Some(vec! [
                Packet {
                    system_id: self.id_other_sytem,
                    priority: 1,
                    sender_id: self.id(),
                    recipient: Recipient::Agent{ agent_id: 0 },
                    message: ProtocolPos::Position(self.pos.0, self.pos.1),
                }
            ])
        }
    }

    struct AgentTestMsgBetweenSystemFactory(u8);

    impl AgentFactory<AgentTestMsgBetweenSystem> for AgentTestMsgBetweenSystemFactory {
        fn create(&self, agent_id: usize) -> AgentTestMsgBetweenSystem {
            AgentTestMsgBetweenSystem {
                id: agent_id,
                pos: (0, 0),
                id_other_sytem: self.0,
            }
        }
    }

    #[test]
    fn it_should_dispatch_message_between_in_other_system() {
        let mut system: AgentSystem<AgentTestMsgBetweenSystem, ProtocolPos>;
        let mut system2: AgentSystem<AgentTestMsgBetweenSystem, ProtocolPos>;
        let id_system = 0;
        let id_system2 = 1;


        system = AgentSystem::new(id_system, Box::new(AgentTestMsgBetweenSystemFactory(id_system2)));
        system2 = AgentSystem::new(id_system2, Box::new(AgentTestMsgBetweenSystemFactory(id_system)));
        let sender = system.get_sender();
        let sender2 = system.get_sender();

        system.spawn_agent();
        system2.spawn_agent();

        system.add_local_system(id_system2, sender2);
        system2.add_local_system(id_system, sender);

        let mut resources = Resources::new();
        let mut dispatcher = DispatcherBuilder::new()
            .add(system, "system 1", &[])
            .add(system2, "system 2", &[])
            .build();

        dispatcher.dispatch(&mut resources);
    }
}