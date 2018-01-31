use slab::Slab;
use shred::System;

use agent::Agent;
use agent_factory::AgentFactory;
use packet::{Packet};

use std::sync::mpsc::{channel, Sender, Receiver};
use std::collections::HashMap;

pub struct AgentSystem<A: Agent, M: Eq> {
    id: u8, 
    agents: Slab<A>,
    inboxes: Vec<Packet<M>>,
    chan: (Sender<Packet<M>>, Receiver<Packet<M>>),
    views: HashMap<u8, Sender<Packet<M>>>,
    factory: Box<AgentFactory<A>>,
}

impl <A: Agent, M: Eq>AgentSystem<A, M> {
    pub fn new(id: u8, factory: Box<AgentFactory<A>>) -> Self {
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
            if let Some(mut messages) = agent.update::<M>() {
                self.inboxes.append(&mut messages);
            }
        }
        self.inboxes.sort();
    }

    pub fn process_messages(&mut self) {
        for packet in self.inboxes.drain(..) {
            if packet.recipient.system_id == self.id {
                let agent = self.agents.get_mut(packet.recipient.agent_id);

                if let Some(agent) = agent {
                    agent.handle_message(packet);
                }
                else {
                    eprintln!("The system {} doesn't contain the agent id {}", self.id, packet.recipient.agent_id);
                }
            }
            else {
                let system_view = self.views.get(&packet.recipient.system_id);

                if let Some(sender) = system_view {
                    //FIXME: Manage the error just by a print.
                    sender.send(packet).expect("Should send the message to the other system");
                }
                else {
                    eprintln!("The system {} is not register in the system {}", packet.recipient.system_id, self.id);
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

    pub fn add_remote_system(&mut self, id: u8, sender: Sender<Packet<M>>) {
        self.views.insert(id, sender);
    }

    pub fn get_nb_message_inbox(&self) -> usize {
        self.inboxes.len()
    }

    pub fn id(&self) -> u8 {
        self.id
    }

    pub fn get_nb_agents_alive(&self) -> usize {
        self.agents.len()
    }
}

impl<'a, A: Agent, M: Eq> System<'a> for AgentSystem<A, M> {
    type SystemData = ();

    fn run(&mut self, _: Self::SystemData) {
        self.get_message_from_other_system();
        self.process_messages();
        self.process_agent();
    }
}

#[cfg(test)]
mod test_sytem {

    use super::*;
    use shred::{DispatcherBuilder, Resource, Resources, System};

    struct Car { 
        id: usize,
        diesel: bool,
    }

    impl Agent for Car {
        fn id(&self) -> usize { self.id }

        fn set_id(&mut self, id: usize) { self.id = id }

        fn is_dead(&self) -> bool { false }

        fn handle_message(&mut self, _: i32) {}

        fn update<MessageType>(&mut self) -> Option<Vec<Packet<MessageType>>> {
            None
        }
    }

    struct Person {
        id: usize,
        age: i32,
    }

    impl Agent for Person {
        fn id(&self) -> usize { self.id }

        fn set_id(&mut self, id: usize) { self.id = id }

        fn is_dead(&self) -> bool { false }

        fn handle_message(&mut self, _: i32) {}

        fn update<MessageType>(&mut self) -> Option<Vec<Packet<MessageType>>> {
            None
        }
    }

    #[derive(Eq, PartialEq)]
    enum MessageType {
        Hello(u8),
        Info(String),
    }

    #[test]
    fn it_should_not_register_the_same_swarm() {
        let mut pers_sys: AgentSystem<Person, MessageType>;
        let mut car_sys: AgentSystem<Car, MessageType>;
        car_sys = AgentSystem::new(0);
        pers_sys = AgentSystem::new(1);

        car_sys.add_remote_system(0, pers_sys.get_sender());

        assert_eq!(0, pers_sys.get_nb_message_inbox());
    }

    #[test]
    fn it_should_run_with_dispatcher() {
        let mut pers_sys: AgentSystem<Person, MessageType>;
        let mut car_sys: AgentSystem<Car, MessageType>;
        car_sys = AgentSystem::new(0);
        pers_sys = AgentSystem::new(1);


        let mut resources = Resources::new();
        let mut dispatcher = DispatcherBuilder::new()
            .add(pers_sys, "person", &[])
            .add(car_sys, "car", &[])
            .build();

        dispatcher.dispatch(&mut resources);
    }
}