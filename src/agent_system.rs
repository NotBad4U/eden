use slab::Slab;

use agent::Agent;
use packet::Packet;

use std::sync::mpsc::{channel, Sender, Receiver};

pub struct AgentSystem<A: Agent, M: Eq> {
    pub id: u8, 
    pub agents: Slab<A>,
    pub inboxes: Vec<Packet<M>>,
    pub chan: (Sender<Packet<M>>, Receiver<Packet<M>>),
    pub views: Vec<(u8, Sender<Packet<M>>)>,
}

impl <A: Agent, M: Eq>AgentSystem<A, M> {
    pub fn new(id: u8) -> Self {
        AgentSystem {
            id,
            agents: Slab::new(),
            inboxes: Vec::new(),
            views: Vec::new(),
            chan: channel(),
        }
    }

    pub fn run(&mut self) {
        self.process_agent();
        self.process_messages();
    }

    pub fn process_agent(&mut self) {
        for (_, agent) in self.agents.iter_mut() {
            if let Some(mut messages) = agent.update::<M>() {
                self.inboxes.append(&mut messages);
            }
        }
    }

    pub fn process_messages(&mut self) {
        for packet in self.chan.1.try_iter() {
            self.inboxes.push(packet);
        }
        self.inboxes.sort();
    }

    pub fn get_sender(&self) -> Sender<Packet<M>> {
        self.chan.0.clone()
    }

    pub fn add_remote_system(&mut self, id: u8, sender: Sender<Packet<M>>) {
        self.views.push((id, sender));
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

#[cfg(test)]
mod test_sytem {

    use super::*;

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

        car_sys.run();
        pers_sys.run();
        assert_eq!(0, pers_sys.get_nb_message_inbox());
    }
}