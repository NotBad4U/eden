use std::cmp::Ordering;

use bincode;
use uuid::Uuid;
use serde::{Serialize, de::DeserializeOwned};

use agent::AgentId;
use agent_system::SystemId;

pub type Id = u8;

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Debug)]
pub enum Recipient {
    Agent { system_id: SystemId, agent_id: AgentId },
    Broadcast { system_id: Option<SystemId> },
}


#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum Performative {
    /// The action of accepting a previously submitted proposal to perform an action.
    AcceptProposal,
    /// The action of agreeing to perform some action, possibly in the future.
    Agree,
    /// The action of one agent informing another agent that the first agent no longer has the intention
    /// that the second agent perform some action.
    Cancel,
    /// The action of calling for proposals to perform a given action.
    CallForProposal,
    /// The sender informs the receiver that a given proposition
    /// is true, where the receiver is known to be uncertain about the proposition.
    Confirm,
    /// The sender informs the receiver that a given proposition is false, where the receiver is known to
    /// believe, or believe it likely that, the proposition is true.
    Disconfirm,
    /// The action of telling another agent that an action was attempted but the attempt failed.
    Failure,
    /// The sender informs the receiver that a given proposition is true.
    Inform,
    /// A macro action for the agent of the action to inform the recipient
    /// whether or not a proposition is true.
    InformIf,
    /// The sender of the act (for example, i) informs the receiver (for example, j)
    /// that it perceived that j performed some action, but that i did not understand what j just did.
    NotUnderstood,
    /// The sender intends that the receiver treat the embedded message as sent directly to the
    /// receiver, and wants the receiver to identify the agents denoted by the given descriptor and send
    /// the received propagate message to them.
    Propagate,
    /// The action of submitting a proposal to perform a certain action, given certain preconditions.
    Propose,
    /// The sender wants the receiver to select target agents denoted
    /// by a given description and to send an embedded message to them.
    Proxy,
    /// The action of asking another agent whether or not a given proposition is true.
    QueryIf,
    /// The action of asking another agent for the object referred to by a referential expression.
    QueryRef,
    /// The action of refusing to perform a given action, and EXPLAINING the reason for the refusal.
    Refuse,
    /// The action of rejecting a proposal to perform some action during a negotiation.
    RejectProposal,
    /// The sender requests the receiver to perform some action.
    /// ( the receiver to perform another communicative act)
    Request,
    /// The sender wants the receiver to perform some action when some given proposition becomes true.
    RequestWhen,
    /// The sender wants the receiver to per²m some action as soon as some proposition becomes true
    /// and thereafter each time the proposition becomes true again.
    RequestWhenever,
    /// The act of requesting a persistent intention to notify the sender of the value of a reference,
    /// and to notify again whenever the object identified by the reference changes.
    Subscribe,
}

pub trait Content: Serialize + DeserializeOwned + Clone {}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Message<C> {
    /// Identifier of the message
    pub id: Uuid,

    /// Define the intention in the message
    pub performative: Performative,

    pub sender: (SystemId, AgentId),

    pub recipient: Recipient,

    /// ontology used to give a meaning to the symbols in the content expression
    pub ontology: u8,

    /// Set the priority level of the message.
    pub priority: u8,

    /// A conversation identifier which is used to identify the ongoing sequence
    /// of communicative acts that together form a conversation.
    pub conversation_id: Option<Id>,

    /// Introduces an expression that will be used by the
    /// responding agent to identify this message.
    pub reply_with: Option<Id>,

    /// Denotes an expression that references an earlier
    /// action to which this message is a reply.
    pub in_reply_to: Option<Id>,

    /// Denotes a time and/or date expression which indicates the latest
    /// time by which the sending agent would like to receive a reply.
    pub reply_by: Option<Id>,

    pub occurred: u64,

    /// Content of the message
    pub content: C,
}


impl<C: Content> Message<C> {
    pub fn new(
        performative: Performative,
        sender: (SystemId, AgentId),
        recipient: Recipient,
        ontology: u8,
        priority: u8,
        conversation_id: Option<Id>,
        reply_with: Option<Id>,
        in_reply_to: Option<Id>,
        reply_by: Option<Id>,
        content: C,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            sender,
            recipient,
            performative,
            ontology,
            priority,
            conversation_id,
            reply_with,
            in_reply_to,
            reply_by,
            content,
            occurred: 0,
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>, Box<bincode::ErrorKind>> {
        bincode::serialize(self)
    }

    pub fn deserialize(msg: &[u8]) -> Result<Self, Box<bincode::ErrorKind>> {
        bincode::deserialize(msg)
    }

    pub fn set_sender(&mut self, sender: (SystemId, AgentId)) {
        self.sender = sender;
    }

    pub fn set_occurred(&mut self , occurred: u64) {
        self.occurred = occurred;
    }
}

impl <C: Content>Ord for Message<C> {
    fn cmp(&self, other: &Message<C>) -> Ordering {
        self.priority.cmp(&other.priority)
    }
}

impl <C: Content>PartialOrd for Message<C> {
    fn partial_cmp(&self, other: &Message<C>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl <C: Content>PartialEq for Message<C> {
    fn eq(&self, other: &Message<C>) -> bool {
        self.id == other.id
    }
}

impl <C: Content>Eq for Message<C> {}

#[cfg(test)]
mod test_message {

    use super::*;

    #[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
    struct Position {
        x: u8,
        y: u8,
    }

    impl Content for Position {}

    #[test]
    fn it_should_cmp_message_priority_to_order_them() {
        let high_priority = 10;
        let low_priority = 0;

        assert!(high_priority > low_priority);

        let message_with_high_priority = Message::new(
            Performative::Confirm,
            (0, 0),
            Recipient::Broadcast{ system_id: None },
            0,
            high_priority,
            None,
            None,
            None,
            None,
            Position{ x: 23, y: 12 },
        );

        let message_with_low_priority = Message::new(
            Performative::Confirm,
            (0, 0),
            Recipient::Broadcast{ system_id: None },
            0,
            low_priority,
            None,
            None,
            None,
            None,
            Position{ x: 23, y: 12 },
        );

        assert_eq!(Ordering::Greater, message_with_high_priority.cmp(&message_with_low_priority));
        assert_eq!(Ordering::Less, message_with_low_priority.cmp(&message_with_high_priority));
    }

    #[test]
    fn it_should_serde_message() {
        let position = Position{ x: 23, y: 12 };

        let message = Message::new(
            Performative::Confirm,
            (0, 0),
            Recipient::Broadcast{ system_id: None },
            0,
            0,
            None,
            None,
            None,
            None,
            position,
        );

        let msg_ser = message.serialize().expect("Should be serialize");
        let msg_deser = Message::deserialize(&msg_ser).expect("Should be deserialize");;

        assert_eq!(message, msg_deser);
    }
}