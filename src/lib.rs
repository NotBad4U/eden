extern crate slab;
extern crate shred;
extern crate zmq;
#[macro_use]
extern crate log;

pub mod agent;
pub mod agent_system;
pub mod agent_factory;
pub mod packet;
pub mod protocol;

mod monitoring;
mod message_collector;
mod inbox;
mod dispatcher;