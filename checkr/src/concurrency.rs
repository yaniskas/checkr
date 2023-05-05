use std::fmt::Display;
use serde::{Serialize, Deserialize};
use itertools::Itertools;

use crate::{pg::{ProgramGraph, Action, Determinism, Node}, interpreter::{Configuration, next_configurations as next_configurations_pg}, ast::ParallelCommands, model_checking::ModelCheckMemory};

#[derive(Debug, Clone)]
pub struct ParallelProgramGraph(pub Vec<ProgramGraph>);

impl ParallelProgramGraph {
    pub fn new(det: Determinism, pcmds: &ParallelCommands) -> ParallelProgramGraph {
        ParallelProgramGraph(pcmds.0.iter().map(|e| ProgramGraph::new(det, e)).collect())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ParallelConfiguration<N = Node> {
    pub nodes: Vec<N>,
    pub memory: ModelCheckMemory,
}

impl Display for ParallelConfiguration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let node_str = format!("{}", self.nodes.iter().join(", "));
        let mut var_strs = self.memory.variables.iter().map(|assignment| {
            format!("{} = {}", assignment.0, assignment.1)
        });
        let mut arr_strs = self.memory.arrays.iter().map(|assignment| {
            let comma_separated = assignment.1.iter().map(ToString::to_string).join(", ");
            format!("{} = [{}]", assignment.0, comma_separated)
        });

        let var_str = var_strs.join(", ");
        let arr_str = arr_strs.join(", ");

        match (&var_str[..], &arr_str[..]) {
            ("", "") => write!(f, "{node_str} {{}}"),
            (var_str, "") => write!(f, "{node_str} {{{}}}", var_str),
            ("", arr_str) => write!(f, "{node_str} {{{}}}", arr_str),
            (var_str, arr_str) => write!(f, "{node_str} {{{}, {}}}", var_str, arr_str)
        }
    }
}

pub fn next_configurations(ppg: &ParallelProgramGraph, config: &ParallelConfiguration) -> Vec<(Action, ParallelConfiguration)> {
    let ParallelConfiguration {nodes: node_vec, memory} = config;

    ppg.0.iter()
        .zip(node_vec)
        .enumerate()
        .flat_map(|(index, (pg, node))| {
            let pg_config = Configuration {node: node.clone(), memory: memory.clone()};

            next_configurations_pg(pg, &pg_config).into_iter()
                .map(move |(action, pg_config)| {
                    let Configuration {node: new_pg_node, memory: new_memory} = pg_config;

                    let mut new_config = ParallelConfiguration {nodes: config.nodes.clone(), memory: new_memory};
                    new_config.nodes[index] = new_pg_node;

                    (action, new_config)
                })
        })
    .collect()
}

#[cfg(test)]
mod test {
    use crate::model_checking::ltl_verification::test::verify_satisfies;

    #[test]
    fn flip_flop() {
        let program = "
        par
            do true -> if i = 0 -> i := 1 fi od 
        [] 
            do true -> if i = 1 -> i := 0 fi od 
        rap
        ";

        verify_satisfies(program, "[]<>{n = 0}");
    }

    #[test]
    fn flip_flop_2() {
        let program = "
        par
            do true -> if i = 0 -> i := 1 fi od 
        [] 
            do true -> if i = 1 -> i := 0 fi od 
        rap
        ";

        verify_satisfies(program, "[]<>{n = 1}");
    }

    #[test]
    fn flip_flop_3() {
        let program = "
        par
            do true -> if i = 0 -> i := 1 fi od 
        [] 
            do true -> if i = 1 -> i := 0 fi od 
        rap
        ";

        verify_satisfies(program, "[]({i = 0} -> <>{i = 1})");
    }
}