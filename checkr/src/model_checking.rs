pub mod ltl_ast;
pub mod vwaa;
pub mod gba;
pub mod simplification;
pub mod ba;
pub mod nested_dfs;
pub mod ltl_verification;

use std::collections::{HashSet, VecDeque, HashMap};

use crate::{interpreter::InterpreterMemory, pg::{Node, Action}, concurrency::{ParallelProgramGraph, ParallelConfiguration, next_configurations}};


pub type ModelCheckMemory = InterpreterMemory;

pub struct CheckedModel {
    pub stuck_states: Vec<ParallelConfiguration>,
    pub transition_system: HashMap<ParallelConfiguration, Vec<(Action, ParallelConfiguration)>>
}

pub fn check_model(
    mut search_depth: u64,
    memory: ModelCheckMemory,
    pg: &ParallelProgramGraph,
) -> CheckedModel {
    let initial_configuration = ParallelConfiguration {
        nodes: pg.initial_nodes(),
        memory
    };

    let mut visited: HashSet<ParallelConfiguration> = HashSet::new();

    let mut transition_system: HashMap<ParallelConfiguration, Vec<(Action, ParallelConfiguration)>> = HashMap::new();

    let mut stuck_states: Vec<ParallelConfiguration> = Vec::new();

    let mut to_explore: VecDeque<ParallelConfiguration> = VecDeque::new();
    to_explore.push_back(initial_configuration);

    while !to_explore.is_empty() {
        if search_depth == 0 { break };

        let mut new_additions = VecDeque::new();

        for cstate in &to_explore {
            if visited.contains(&cstate) { continue };

            visited.insert(cstate.clone());

            let potential_next_states = next_configurations(pg, &cstate);

            if !potential_next_states.is_empty() {
                new_additions.extend(potential_next_states.iter().map(|e| e.1.clone()));
                transition_system.insert(
                    cstate.clone(),
                    potential_next_states
                );
            } else if !cstate.nodes.iter().all(|n| *n == Node::End) {
                stuck_states.push(cstate.clone());
            }
        }

        to_explore = new_additions;

        search_depth -= 1;
    }

    CheckedModel { stuck_states, transition_system }
}

// impl Display for Configuration<Node> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         let node_str = format!("{}", self.node);
//         let mut var_strs = self.memory.variables.iter().map(|assignment| {
//             format!("{} = {}", assignment.0, assignment.1)
//         });
//         let mut arr_strs = self.memory.arrays.iter().map(|assignment| {
//             let comma_separated = assignment.1.iter().map(ToString::to_string).join(", ");
//             format!("{} = [{}]", assignment.0, comma_separated)
//         });

//         let var_str = var_strs.join(", ");
//         let arr_str = arr_strs.join(", ");

//         match (&var_str[..], &arr_str[..]) {
//             ("", "") => write!(f, "{node_str} {{}}"),
//             (var_str, "") => write!(f, "{node_str} {{{}}}", var_str),
//             ("", arr_str) => write!(f, "{node_str} {{{}}}", arr_str),
//             (var_str, arr_str) => write!(f, "{node_str} {{{}, {}}}", var_str, arr_str)
//         }
//     }
// }