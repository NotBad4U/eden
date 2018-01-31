use agent::Agent;

pub trait AgentFactory<A: Agent> {

    fn create(&self, agent_id: usize) -> A;

}