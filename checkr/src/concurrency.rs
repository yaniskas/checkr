use std::fmt::Display;
use serde::{Serialize, Deserialize};
use itertools::Itertools;

use crate::{pg::{ProgramGraph, Action, Determinism, Node}, interpreter::{Configuration, next_configurations as next_configurations_pg}, ast::ParallelCommands, model_checking::ModelCheckMemory};

#[derive(Debug, Clone)]
pub struct ParallelProgramGraph(pub Vec<ProgramGraph>);

impl ParallelProgramGraph {
    pub fn dot(&self) -> String {
        format!(
            "digraph G {{\n{}\n}}",
            self.0.iter().enumerate().flat_map(|(i, pg)| pg.edges().iter().map(move |edges| (i, edges)))
            .map(|(i, e)| format!(
                    "  {:?}[label=\"{}\"]; {:?} -> {:?}[label={:?}]; {:?}[label=\"{}\"];",
                    format!("{}, {}", i, e.0),
                    format!("{}, {}", i, e.0),
                    format!("{}, {}", i, e.0),
                    format!("{}, {}", i, e.2),
                    e.1.to_string(),
                    format!("{}, {}", i, e.2),
                    format!("{}, {}", i, e.1),
                ))
                .format("  \n")
        )
    }
}

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
                .map(move |(action, new_pg_config)| {
                    let Configuration {node: new_pg_node, memory: new_memory} = new_pg_config;

                    let mut new_config = ParallelConfiguration {nodes: config.nodes.clone(), memory: new_memory};
                    new_config.nodes[index] = new_pg_node;

                    (action, new_config)
                })
        })
    .collect()
}

#[cfg(test)]
mod test {
    use crate::model_checking::ltl_verification::test::{verify_satisfies, verify_not_satisfies, verify_satisfies_name};

    #[test]
    fn flip_flop() {
        let program = "
        par
            do true -> if i = 0 -> i := 1 fi od 
        [] 
            do true -> if i = 1 -> i := 0 fi od 
        rap
        ";

        verify_satisfies(program, "[]<>{i = 0}");
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

        verify_satisfies(program, "[]<>{i = 1}");
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

    #[test]
    fn peterson() {
        let program = "
        par
            do true ->
                in1 := 1;
                turn := 2;

                if in2 = 0 || turn = 1 -> skip fi;

                incrit := incrit + 1;
                incrit := incrit - 1;

                in1 := 0
            od
        [] 
            do true ->
                in2 := 1;
                turn := 1;

                if in1 = 0 || turn = 2 -> skip fi;

                incrit := incrit + 1;
                incrit := incrit - 1;

                in2 := 0
            od
        rap
        ";

        verify_satisfies(program, "[]{incrit < 2}");
    }

    #[test]
    fn peterson_obligingness() {
        let program = "
        par
            do true ->
                entry1 := 1;
                in1 := 1;
                entry1 := 0;
                turn := 2;

                if in2 = 0 || turn = 1 -> skip fi;

                crit1 := 1;
                incrit := incrit + 1;
                crit1 := 0;
                incrit := incrit - 1;

                in1 := 0
            od
        [] 
            do true ->
                entry2 := 1;
                in2 := 1;
                entry2 := 0;
                turn := 1;

                if in1 = 0 || turn = 2 -> skip fi;

                incrit := incrit + 1;
                incrit := incrit - 1;

                in2 := 0
            od
        rap
        ";

        verify_satisfies(program, "[]( ({entry1 = 1} && []!{entry2 = 1}) -> <>{crit1 = 1} )")
    }

    #[test]
    fn peterson_resolution() {
        let program = "
        par
            do true ->
                entry1 := 1;
                in1 := 1;
                entry1 := 0;
                turn := 2;

                if in2 = 0 || turn = 1 -> skip fi;

                crit1 := 1;
                incrit := incrit + 1;
                crit1 := 0;
                incrit := incrit - 1;

                in1 := 0
            od
        [] 
            do true ->
                entry2 := 1;
                in2 := 1;
                entry2 := 0;
                turn := 1;

                if in1 = 0 || turn = 2 -> skip fi;

                incrit := incrit + 1;
                incrit := incrit - 1;

                in2 := 0
            od
        rap
        ";

        verify_satisfies(program, "[]( ({entry1 = 1} || {entry2 = 1}) -> <>({crit1 = 1} || {crit2 = 1}) )");
    }

    #[test]
    fn peterson_not_fair() {
        let program = "
        par
            do true ->
                entry1 := 1;
                in1 := 1;
                entry1 := 0;
                turn := 2;

                if in2 = 0 || turn = 1 -> skip fi;

                crit1 := 1;
                incrit := incrit + 1;
                crit1 := 0;
                incrit := incrit - 1;

                in1 := 0
            od
        [] 
            do true ->
                entry2 := 1;
                in2 := 1;
                entry2 := 0;
                turn := 1;

                if in1 = 0 || turn = 2 -> skip fi;

                incrit := incrit + 1;
                incrit := incrit - 1;

                in2 := 0
            od
        rap
        ";

        verify_not_satisfies(program, "[]( {entry1 = 1} -> <>{crit1 = 1} )");
    }

    #[test]
    fn peterson_not_fair_2() {
        let program = "
        par
            do true ->
                entry1 := 1;
                in1 := 1;
                entry1 := 0;
                turn := 2;

                if in2 = 0 || turn = 1 -> skip fi;

                crit1 := 1;
                incrit := incrit + 1;
                crit1 := 0;
                incrit := incrit - 1;

                in1 := 0
            od
        [] 
            do true ->
                entry2 := 1;
                in2 := 1;
                entry2 := 0;
                turn := 1;

                if in1 = 0 || turn = 2 -> skip fi;

                crit2 := 1;
                incrit := incrit + 1;
                crit2 := 0;
                incrit := incrit - 1;

                in2 := 0
            od
        rap
        ";

        verify_not_satisfies(program, "[]( {entry2 = 1} -> <>{crit2 = 1} )");
    }

    #[test]
    fn atomic() {
        let program = "
        do true ->
            ato x := 1; x := 2 ota
        od
        ";

        verify_satisfies(program, "[]!{x = 1}");
    }

    #[test]
    fn cond_atomic() {
        let program = "
        do true ->
            ato x = 0 -> x := 1
            [] x = 1 -> x := 0
            ota
        od
        ";

        verify_satisfies(program, "[](({x = 0} -> ()(){x = 1}) && ({x = 1} -> ()(){x = 0}))");
    }
}