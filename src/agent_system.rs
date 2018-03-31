use slab::Slab;
use shred::System;
use zmq::Context as ZmqContext;
use serde::{Serialize, de::DeserializeOwned};

use message::*;
use agent::Agent;
use agent_factory::AgentFactory;
use dispatcher::Dispatcher;
use message_collector::Collector;

use std::sync::mpsc::{channel, Sender};
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH, Duration};

pub type SystemId = u8;

pub struct AgentSystem<A: Agent<Content=C>, C: Serialize + DeserializeOwned + Clone + Eq> {
    id: SystemId,
    agents: Slab<A>,
    outbox: Vec<Message<C>>,
    sender: Sender<Message<C>>,
    factory: Box<AgentFactory<A> + Send>,
    dispatcher: Dispatcher<C>,
    collector: Collector<C>,
}

impl <A: Agent<Content=C>, C: Serialize + DeserializeOwned + Clone + Eq>AgentSystem<A, C> {

    pub fn new(id: SystemId, factory: Box<AgentFactory<A> + Send>, addr: SocketAddr) -> Self {
        trace!("Creating the system {}", id);

        let zmq_ctx = ZmqContext::new();
        let (sender, receiver) = channel();
        let dispatcher = Dispatcher::<C>::new(&zmq_ctx, addr);
        let collector = Collector::<C>::new(id, zmq_ctx, receiver, None);

        let mut agent_system = AgentSystem {
            id,
            agents: Slab::new(),
            outbox: Vec::new(),
            sender: sender.clone(),
            factory,
            dispatcher,
            collector,
        };

        // Register itself to dispatch the message to the same agents.
        agent_system.add_local_observer_system(id, sender);

        agent_system
    }


    pub fn spawn_agent(&mut self) {
        trace!("Creating an agent on system {}", self.id());
        let entry_agent = self.agents.vacant_entry();
        let agent = self.factory.create(entry_agent.key());
        entry_agent.insert(agent);
    }

    pub fn spawn_swarm(&mut self, count: usize) {
        trace!("Creating {} agent on system {}", count, self.id());
        for _ in 0..count {
            self.spawn_agent();
        }
    }

    pub fn process_agent(&mut self) {
        let sid = self.id();
        let occurred = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or(Duration::new(0,0));

        for (_, a) in self.agents.iter_mut() {
            if let Some(mut messages) = a.act() {
                for m in messages.iter_mut() {
                    m.set_occurred(occurred.as_secs());
                }

                self.outbox.append(&mut messages);
            }
        }
        self.outbox.sort();
    }

    pub fn send_agents_messages(&mut self) {
        let messages = self.outbox.drain(..);
        self.dispatcher.dispatch_messages(messages);
    }

    pub fn collect_messages(&mut self) {
        self.collector.collect_messages();
    }

    pub fn distribute_messages_collected_to_the_agents(&mut self) {
        let sys_id = self.id();

        if let Some(messages) = self.collector.drain_inbox() {
            for m in messages {
                match m.recipient {
                    Recipient::Agent{ system_id: _, agent_id } => {
                        if let Some(agent) = self.agents.get_mut(agent_id) {
                            if agent.id() != m.sender.1 || sys_id != m.sender.0 {
                                agent.handle_message(&m);
                            }
                        }
                    },
                    Recipient::Broadcast{ system_id: _ } => {
                        self.agents
                            .iter_mut()
                            .filter(|&(_,ref agent)|
                                agent.id() != m.sender.1
                                || sys_id != m.sender.0)
                            .for_each(|(_, agent)| agent.handle_message(&m));
                    }
                }
            }
        }
    }

    pub fn add_local_observer_system(&mut self, system_id: SystemId, channel_sender: Sender<Message<C>>) {
        trace!("Adding the local observer system {}", system_id);
        self.dispatcher.add_local_sender(system_id, channel_sender);
    }

    pub fn add_remote_observer_system(&mut self, remotes_system: (SystemId, SocketAddr)) {
        trace!("Adding the remote observer system {} - {}", remotes_system.0, remotes_system.1);
        self.collector.add_remote_collector(remotes_system.1);
    }

    #[inline]
    pub fn get_sender(&self) -> Sender<Message<C>> {
        self.sender.clone()
    }

    #[inline]
    pub fn get_nb_message_inbox(&self) -> usize {
        self.outbox.len()
    }

    #[inline]
    pub fn id(&self) -> u8 {
        self.id
    }

    #[inline]
    pub fn get_nb_agents(&self) -> usize {
        self.agents.len()
    }
}

impl<'a, A: Agent<Content=C>, C: Serialize + DeserializeOwned + Clone + Eq>System<'a> for AgentSystem<A, C> {
    type SystemData = ();

    fn run(&mut self, _: Self::SystemData) {
        self.process_agent();
        self.send_agents_messages();
        self.collect_messages();
        self.distribute_messages_collected_to_the_agents();
    }
}

#[cfg(test)]
mod test_sytem {

    use super::*;
    use packet::Recipient;
    use agent::Message;

    use shred::{DispatcherBuilder, Resources};

    use std::net::ToSocketAddrs;

    struct Person {
        id: usize,
    }

    #[derive(Clone, Eq, PartialEq)]
    enum Protocol{
        Foo,
    }

    impl Payload for Protocol {
        fn deserialize(bytes: &[u8]) -> Result<Self, ()> { Ok(Protocol::Foo) }

        fn serialize(&self) -> Vec<u8> { unimplemented!() }
    }

    impl Agent for Person {
        type P = Protocol;

        fn id(&self) -> usize { self.id }

        fn set_id(&mut self, id: usize) { self.id = id }

        fn is_dead(&self) -> bool { false }

        fn handle_message(&mut self, _: &Message<Self::P>) {}

        fn act(&mut self) -> Option<Vec<Message<Self::P>>> {
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
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        pers_sys = AgentSystem::new(0, Box::new(PersonFactory), addr);

        pers_sys.spawn_agent();
        pers_sys.spawn_agent();

        assert_eq!(2, pers_sys.get_nb_agents());
    }

    #[test]
    fn it_should_spawn_a_swarm_of_agents() {
        let nb_agent_to_spawn = 10;
        let mut pers_sys: AgentSystem<Person, Protocol>;
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));

        pers_sys = AgentSystem::new(0, Box::new(PersonFactory), addr);

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

    impl Payload for ProtocolGreeting {
        fn deserialize(bytes: &[u8]) -> Result<Self, ()> { Ok(ProtocolGreeting::Greeting(0)) }

        fn serialize(&self) -> Vec<u8> { vec![] }
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

        fn act(&mut self) -> Option<Vec<Message<Self::P>>> {
            Some(vec! [
                Message {
                    priority: 1,
                    recipient: Recipient::Agent{ agent_id: 1 - self.id(), system_id: 0 },
                    message: ProtocolGreeting::Greeting(self.id()),
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
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));

        system = AgentSystem::new(0, Box::new(AgentTestMsgFactory), addr);
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
                ProtocolGreeting::Greeting(id) => {
                    println!("I'm {} and I got Hello from {}", self.id(), id);
                },
            }
        }

        fn act(&mut self) -> Option<Vec<Message<Self::P>>> {
            Some(vec! [
                Message {
                    priority: 1,
                    recipient: Recipient::Broadcast{ system_id: None },
                    message: ProtocolGreeting::Greeting(self.id()),
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
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));

        system = AgentSystem::new(0, Box::new(GentTestMsgBroadcastFactory), addr);
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

    impl Payload for ProtocolPos {
        fn deserialize(bytes: &[u8]) -> Result<Self, ()> { Ok(ProtocolPos::Position(0, 0)) }

        fn serialize(&self) -> Vec<u8> { unimplemented!() }
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

        fn act(&mut self) -> Option<Vec<Message<Self::P>>> {
            Some(vec! [
                Message {
                    priority: 1,
                    recipient: Recipient::Agent{ agent_id: 0, system_id: self.id_other_sytem },
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
        let addr_system = SocketAddr::from(([127, 0, 0, 1], 0));
        let addr_system2 = SocketAddr::from(([127, 0, 0, 1], 0));


        system = AgentSystem::new(id_system, Box::new(AgentTestMsgBetweenSystemFactory(id_system2)), addr_system);
        system2 = AgentSystem::new(id_system2, Box::new(AgentTestMsgBetweenSystemFactory(id_system)), addr_system2);
        let sender = system.get_sender();
        let sender2 = system.get_sender();

        system.spawn_agent();
        system2.spawn_agent();

        system.add_local_observer_system(id_system2, sender2);
        system2.add_local_observer_system(id_system, sender);

        let mut resources = Resources::new();
        let mut dispatcher = DispatcherBuilder::new()
            .add(system, "system 1", &[])
            .add(system2, "system 2", &[])
            .build();

        dispatcher.dispatch(&mut resources);
    }
}