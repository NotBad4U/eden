#[macro_use]
extern crate log;
extern crate shred;
extern crate slab;
extern crate uuid;
extern crate zmq;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate bincode;

pub mod agent;
pub mod agent_system;
pub mod agent_factory;
pub mod message;

mod monitoring;
mod message_collector;
mod dispatcher;
mod utils;