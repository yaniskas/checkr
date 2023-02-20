use std::collections::{HashSet, VecDeque};

use crate::{interpreter::{ProgramTrace, InterpreterMemory, ProgramState, next_states}, pg::{ProgramGraph, Node}};


pub type ModelCheckMemory = InterpreterMemory;

pub type Configuration = ProgramTrace;

pub fn stuck_states(
    mut search_depth: u64,
    memory: ModelCheckMemory,
    pg: &ProgramGraph,
) -> Vec<Configuration> {
    let initial_configuration = Configuration {
        state: ProgramState::Running,
        node: Node::Start,
        memory
    };

    let mut visited: HashSet<Configuration> = HashSet::new();

    let mut stuck_states: Vec<Configuration> = Vec::new();

    let mut to_explore: VecDeque<Configuration> = VecDeque::new();
    to_explore.push_back(initial_configuration);

    while let Some(cstate) = to_explore.pop_front() {
        if visited.contains(&cstate) { continue };

        visited.insert(cstate.clone());

        let potential_next_states = next_states(pg, &cstate);

        if potential_next_states.is_empty() {
            stuck_states.push(cstate);
        } else {
            to_explore.extend(potential_next_states.into_iter());
        }

        search_depth -= 1;
        if search_depth == 0 { break };
    }

    stuck_states
}