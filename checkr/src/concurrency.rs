use std::{fmt::Display, collections::{HashSet, HashMap}};
use serde::{Serialize, Deserialize};
use itertools::Itertools;

use crate::{pg::{ProgramGraph, Action, Determinism, Node, Edge}, interpreter::{Configuration, next_configurations as next_configurations_pg}, ast::ParallelCommands, model_checking::{ModelCheckMemory}, util::traits::AddMany};

#[derive(Debug, Clone)]
pub struct ParallelProgramGraph(pub Vec<ProgramGraph>);

impl ParallelProgramGraph {
    pub fn to_pg(self) -> ProgramGraph {
        let (edges, nodes, outgoing) = self.0.into_iter()
            .fold((Vec::new(), HashSet::new(), HashMap::new()), |(acc_edges, acc_nodes, acc_outgoing), pg| {
                (
                    acc_edges.add_many(pg.edges),
                    acc_nodes.add_many(pg.nodes),
                    acc_outgoing.add_many(pg.outgoing)
                )
            });
        ProgramGraph {edges, nodes, outgoing}
    }

    pub fn dot(&self) -> String {
        if self.num_processes() == 1 {
            let non_parallel_nodes = self.0[0].nodes.iter().map(Node::to_non_parallel).collect();
            let non_parallel_edges = self.0[0].edges.iter().map(Edge::to_non_parallel).collect();
            let non_parallel_outgoing = self.0[0].outgoing.iter()
                .map(|(n, edges)| (n.to_non_parallel(), edges.iter().map(Edge::to_non_parallel).collect()))
                .collect();

            ProgramGraph {
                nodes: non_parallel_nodes,
                edges: non_parallel_edges,
                outgoing: non_parallel_outgoing
            }.dot()
        } else {
            self.clone().to_pg().dot()
        }
    }

    pub fn num_processes(&self) -> usize {
        self.0.len()
    }

    pub fn initial_nodes(&self) -> Vec<Node> {
        (0..self.num_processes()).into_iter().map(|i| Node::ParallelStart(i as u64)).collect()
    }

    pub fn new(det: Determinism, pcmds: &ParallelCommands) -> ParallelProgramGraph {
        let graphs = pcmds.0.iter()
            .enumerate()
            .map(|(i, e)| {
                let i = i as u64;
                let ProgramGraph {edges, nodes, outgoing} = ProgramGraph::new(det, e);

                let edges = edges.into_iter().map(|e| e.to_parallel(i)).collect();
                let nodes = nodes.into_iter().map(|n| n.to_parallel(i)).collect();
                let outgoing = outgoing.into_iter().map(|(node, edges)| {
                    (node.to_parallel(i), edges.into_iter().map(|e| e.to_parallel(i)).collect())
                }).collect();

                ProgramGraph {edges, nodes, outgoing}
            })
            .collect();

        ParallelProgramGraph(graphs)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ParallelConfiguration<N = Node> {
    pub nodes: Vec<N>,
    pub memory: ModelCheckMemory,
}

impl ParallelConfiguration<Node> {
    pub fn make_node_non_parallel(self) -> ParallelConfiguration {
        ParallelConfiguration {
            nodes: vec![self.nodes[0].to_non_parallel()],
            memory: self.memory
        }
    }
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
    use crate::model_checking::ltl_verification::test::{verify_satisfies, verify_not_satisfies, verify_satisfies_det};

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

    #[test]
    fn nondet_atomic() {
        let program = "
        x := 3;
        ato x = 3 -> y := 0
        [] true -> y := 1
        ota
        ";

        verify_not_satisfies(program, "[]{y = 0}");
    }

    #[test]
    fn det_atomic() {
        let program = "
        x := 3;
        ato x = 3 -> y := 0
        [] true -> y := 1
        ota
        ";

        verify_satisfies_det(program, "[]{y = 0}");
    }
}