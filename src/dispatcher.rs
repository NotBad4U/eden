use zmq::{Socket, Context as ZmqContext, PUB};

use message::*;
use agent_system::SystemId;

use std::{
    collections::HashMap,
    sync::mpsc::Sender,
    net::SocketAddr,
    vec::Drain,
};

const NO_FLAGS: i32 = 0;

const SEND_TO_AGENT: u8 = 0;
const BROADCAST_TO_SYSTEM: u8 = 1;
const BROADCAST_TO_ALL: u8 = 2;

pub struct Dispatcher<C: Content> {
    local_observers: HashMap<u8, Sender<Message<C>>>,
    broadcast_publisher: Socket,
}

macro_rules! log_if_error {
    ($e: expr) => {
        if let Err(e) = $e {
            error!("{}", e);
        }
    }
}

impl <C: Content>Dispatcher<C> {

    pub fn new(zmq_ctx: &ZmqContext, addr: SocketAddr) -> Self {
        let broadcast_publisher = create_publisher_chan_for_broadcast(&zmq_ctx, addr);

        Dispatcher {
            local_observers: HashMap::new(),
            broadcast_publisher,
        }
    }

    pub fn add_local_sender(&mut self, sys_id: u8, sender: Sender<Message<C>>) {
        self.local_observers.insert(sys_id, sender);
    }

    pub fn dispatch_messages(&self, messages: Drain<Message<C>>) {
        for m in messages {
            match m.recipient {
                Recipient::Agent{ system_id, agent_id: _ }
                | Recipient::Broadcast{ system_id: Some(system_id) } => {
                    if self.is_a_message_for_a_local_system(&m) {
                        self.forward_message_to_local_sytem(m, system_id);
                    } else {
                        self.forward_message_to_remote_sytem(&m);
                    }
                },
                Recipient::Broadcast{ system_id: None } => {
                    self.forward_message_to_remote_sytem(&m);
                    self.broadcast_message_to_local_systems(&m);
                },
            }
        }
    }

    fn forward_message_to_local_sytem(&self, message: Message<C>, system_id: SystemId) {
        if let Some(observer) = self.local_observers.get(&system_id) {
            debug!("send a message to a agent in the local system {}", system_id);
            log_if_error!(observer.send(message))
        }
    }

    fn broadcast_message_to_local_systems(&self, message: &Message<C>) {
        for (_, observer) in self.local_observers.iter() {
            debug!("broadcast a message to all local observers systems");
            log_if_error!(observer.send(message.clone()))
        }
    }

    // As we use zmq PUB/SUB, it's the same method for broadcasting or target message
    fn forward_message_to_remote_sytem(&self, message: &Message<C>) {
        let filter_zmq = get_zmq_filter(message);

        if let Ok(msg) = message.serialize() {
            log_if_error!(self.broadcast_publisher
                            .send_multipart(
                                &[
                                    &filter_zmq[..],
                                    msg.as_slice(),
                                ]
                                , NO_FLAGS)
                            )
        }
        else {
            error!("Error during serialize");
        }
    }

    fn is_a_message_for_a_local_system(&self, message: &Message<C>) -> bool {
        match message.recipient {
            Recipient::Agent{ system_id, agent_id: _ } if self.local_observers.contains_key(&system_id) => true,
            Recipient::Broadcast{ system_id } => {
                if let Some(system_id) = system_id {
                    return self.local_observers.contains_key(&system_id)
                }
                false
            },
            _ => false,
        }
    }
}

fn get_zmq_filter<C>(message: &Message<C>) -> [u8; 2] {
    match message.recipient {
        Recipient::Agent{ system_id, agent_id: _ } => [ SEND_TO_AGENT, system_id ],
        Recipient::Broadcast{ system_id: Some(system_id) } => [ BROADCAST_TO_SYSTEM, system_id ],
        Recipient::Broadcast{ system_id: None } => [ BROADCAST_TO_ALL, 0 ],
    }
}

fn create_publisher_chan_for_broadcast(zmq_ctx: &ZmqContext, addr: SocketAddr) -> Socket {
    //TODO: Manage errors
    let zmq_publisher = zmq_ctx.socket(PUB).unwrap();
    zmq_publisher.bind(&format!("tcp://{}", addr)).expect("failed binding publisher");
    info!("Remote publisher is ready to send message on {}", addr);

    zmq_publisher
}


#[cfg(test)]
mod test {
    use super::*;
    use std::sync::mpsc;

    #[derive(Serialize, Deserialize, Clone, Debug)]
    struct EmptyPayload {}

    impl Content for EmptyPayload {}

    #[test]
    fn it_should_detect_that_is_a_message_for_a_remote_system() {
        let message = Message::new(
            Performative::Confirm,
            Recipient::Broadcast{ system_id: None },
            3,
            0,
            None,
            None,
            None,
            None,
            EmptyPayload{},
        );

        let zmq_ctx = ZmqContext::new();
        let addr = "127.0.0.1:8080".parse().expect("Addr error");
        let dispatcher = Dispatcher::new(&zmq_ctx, addr);

        assert_eq!(false, dispatcher.is_a_message_for_a_local_system(&message));
    }

    #[test]
    fn it_should_detect_that_is_a_message_for_a_local_known_system() {
        let local_system_id = 1;

        let message = Message::new(
            Performative::Confirm,
            Recipient::Broadcast{ system_id: Some(local_system_id) },
            3,
            0,
            None,
            None,
            None,
            None,
            EmptyPayload{},
        );

        let zmq_ctx = ZmqContext::new();
        let addr = "127.0.0.1:8081".parse().expect("Addr error");
        let mut dispatcher = Dispatcher::new(&zmq_ctx, addr);

        dispatcher.add_local_sender(local_system_id, mpsc::channel().0);

        assert!(true, dispatcher.is_a_message_for_a_local_system(&message));
    }

    #[test]
    fn it_should_detect_that_his_a_message_for_a_remote_system() {
        let message = Message::new(
            Performative::Confirm,
            Recipient::Broadcast{ system_id: Some(255) },
            3,
            0,
            None,
            None,
            None,
            None,
            EmptyPayload{},
        );

        let zmq_ctx = ZmqContext::new();
        let addr = "127.0.0.1:8082".parse().expect("Addr error");
        let dispatcher = Dispatcher::new(&zmq_ctx, addr);

        assert_eq!(false, dispatcher.is_a_message_for_a_local_system(&message));
    }

    #[test]
    fn it_should_detect_that_his_a_message_for_an_agent_in_a_remote_system() {
        let message = Message::new(
            Performative::Confirm,
            Recipient::Agent{ system_id: 42, agent_id: 42 },
            3,
            0,
            None,
            None,
            None,
            None,
            EmptyPayload{},
        );

        let zmq_ctx = ZmqContext::new();
        let addr = "127.0.0.1:8083".parse().expect("Addr error");
        let dispatcher = Dispatcher::new(&zmq_ctx, addr);

        assert_eq!(false, dispatcher.is_a_message_for_a_local_system(&message));
    }

    #[test]
    fn it_should_detect_that_his_a_message_for_an_agent_in_a_local_system() {
        let local_system_id = 1;

        let message = Message::new(
            Performative::Confirm,
            Recipient::Agent{ system_id: local_system_id, agent_id: 0 },
            3,
            0,
            None,
            None,
            None,
            None,
            EmptyPayload{},
        );

        let zmq_ctx = ZmqContext::new();
        let addr = "127.0.0.1:8084".parse().expect("Addr error");
        let mut dispatcher = Dispatcher::new(&zmq_ctx, addr);

        dispatcher.add_local_sender(local_system_id, mpsc::channel().0);

        assert!(dispatcher.is_a_message_for_a_local_system(&message));
    }
}