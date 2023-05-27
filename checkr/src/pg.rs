use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
};

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{ast::{BExpr, Command, Commands, Guard, LogicOp, Target, Assignment, AtomicStatement, SimpleCommands, AtomicGuard, AtomicGuards}, concurrency::ParallelProgramGraph, util::traits::AddMany};

#[derive(Debug, Clone)]
pub struct ProgramGraph {
    pub edges: Vec<Edge>,
    pub nodes: HashSet<Node>,
    pub outgoing: HashMap<Node, Vec<Edge>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "Case")]
pub enum Determinism {
    Deterministic,
    NonDeterministic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeId(u64);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Node {
    Start,
    Node(NodeId),
    End,
    // Needed so the interface does not have to handle regular program graphs and parallel program graphs separately
    ParallelStart(u64),
    ParallelNode(u64, NodeId),
    ParallelEnd(u64)
}

impl Node {
    pub fn to_parallel(&self, process_num: u64) -> Node {
        match self {
            Node::Start => Node::ParallelStart(process_num),
            Node::Node(n) => Node::ParallelNode(process_num, n.clone()),
            Node::End => Node::ParallelEnd(process_num),
            Node::ParallelStart(_)
            | Node::ParallelNode(_, _)
            | Node::ParallelEnd(_) => panic!("Node is already parallel")
        }
    }

    pub fn to_non_parallel(&self) -> Node {
        match self {
            Node::Start | Node::Node(_) | Node::End => panic!("Node is already non-parallel"),
            Node::ParallelStart(_) => Node::Start,
            Node::ParallelNode(_, id) => Node::Node(*id),
            Node::ParallelEnd(_) => Node::End,
        }
    }
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Node::Start => write!(f, "qStart"),
            Node::Node(n) => write!(f, "q{}", n.0),
            Node::End => write!(f, "qFinal"),
            Node::ParallelStart(i) => write!(f, "qStart_{i}"),
            Node::ParallelNode(i, n) => write!(f, "q{}_{}", n.0, i),
            Node::ParallelEnd(i) => write!(f, "qFinal_{i}"),
        }
    }
}
impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Node::Start => write!(f, "q▷"),
            Node::Node(n) => write!(
                f,
                "q{}",
                n.0,
            ),
            Node::End => write!(f, "q◀"),
            Node::ParallelStart(i) => write!(f, "q▷ {i}"),
            Node::ParallelNode(i, n) => write!(
                f,
                "q{} {}",
                n.0,
                i
            ),
            Node::ParallelEnd(i) => write!(f, "q◀ {i}"),
        }
    }
}

pub struct NodeFactory {
    next_id: u64,
}

impl NodeFactory {
    pub fn new() -> Self {
        NodeFactory {next_id: 0}
    }

    pub fn fresh(&mut self) -> Node {
        let res = Node::Node(NodeId(self.next_id));
        self.next_id += 1;
        res
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Action {
    Assignment(Assignment),
    Atomic(SimpleCommands),
    ConditionalAtomic(BExpr, SimpleCommands),
    Skip,
    Condition(BExpr),
}
impl Action {
    fn fv(&self) -> HashSet<Target> {
        match self {
            Action::Assignment(a) => a.fv(),
            Action::Atomic(commands) => commands.fv(),
            Action::ConditionalAtomic(b, commands) => b.fv().add_many(commands.fv()),
            Action::Skip => Default::default(),
            Action::Condition(b) => b.fv(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Edge(pub Node, pub Action, pub Node);

impl Edge {
    pub fn action(&self) -> &Action {
        &self.1
    }

    pub fn from(&self) -> Node {
        self.0
    }
    pub fn to(&self) -> Node {
        self.2
    }

    pub fn to_parallel(&self, process_num: u64) -> Edge {
        let Edge(s, a, t) = self;
        Edge(
            s.to_parallel(process_num),
            a.clone(),
            t.to_parallel(process_num)
        )
    }

    pub fn to_non_parallel(&self) -> Edge {
        let Edge(s, a, t) = self;
        Edge(
            s.to_non_parallel(),
            a.clone(),
            t.to_non_parallel()
        )
    }
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Assignment(a) => a.fmt(f),
            Action::Atomic(commands) => write!(f, "{}", commands.0.iter().format("; ")),
            Action::ConditionalAtomic(b, commands) => write!(f, "{} -> {}", b, commands.0.iter().format("; ")),
            Action::Skip => write!(f, "skip"),
            Action::Condition(b) => write!(f, "{b}"),
        }
    }
}

impl Commands {
    fn edges(&self, det: Determinism, s: Node, t: Node, node_factory: &mut NodeFactory) -> Vec<Edge> {
        let mut edges = vec![];

        let mut prev = s;
        for (idx, cmd) in self.0.iter().enumerate() {
            let is_last = idx + 1 == self.0.len();
            let next = if is_last { t } else { node_factory.fresh() };
            edges.extend(cmd.edges(det, prev, next, node_factory));
            prev = next;
        }

        edges
    }
}

/// Computes the edges and the condition which is true iff all guards are false
fn guard_edges(det: Determinism, guards: &[Guard], s: Node, t: Node, node_factory: &mut NodeFactory) -> (Vec<Edge>, BExpr) {
    match det {
        Determinism::Deterministic => {
            // See the "if" and "do" Commands on Page 25 of Formal Methods
            let mut prev = BExpr::Bool(false);

            let mut edges = vec![];

            for Guard(b, c) in guards {
                let q = node_factory.fresh();

                edges.push(Edge(
                    s,
                    Action::Condition(BExpr::logic(
                        b.clone(),
                        LogicOp::Land,
                        BExpr::Not(Box::new(prev.clone())),
                    )),
                    q,
                ));
                edges.extend(c.edges(det, q, t, node_factory));
                prev = BExpr::logic(b.to_owned().clone(), LogicOp::Lor, prev);
            }

            // Wraps in "not" so that the "d" part can be used directly by "do"
            (edges, BExpr::Not(Box::new(prev)))
        }
        Determinism::NonDeterministic => {
            let e = guards
                .iter()
                .flat_map(|Guard(b, c)| {
                    let q = node_factory.fresh();
                    let mut edges = c.edges(det, q, t, node_factory);
                    edges.push(Edge(s, Action::Condition(b.clone()), q));
                    edges
                })
                .collect();
            (e, done(guards))
        }
    }
}

impl Command {
    fn edges(&self, det: Determinism, s: Node, t: Node, node_factory: &mut NodeFactory) -> Vec<Edge> {
        match self {
            Command::Assignment(Assignment(v, expr)) => {
                vec![Edge(s, Action::Assignment(Assignment(v.clone(), expr.clone())), t)]
            }
            Command::Skip => vec![Edge(s, Action::Skip, t)],
            Command::If(guards) => guard_edges(det, guards, s, t, node_factory).0,
            Command::Loop(guards) | Command::EnrichedLoop(_, guards) => {
                let (mut edges, b) = guard_edges(det, guards, s, s, node_factory);
                edges.push(Edge(s, Action::Condition(b), t));
                edges
            }
            Command::Atomic(statement) => statement.edges(det, s, t),
            Command::Annotated(_, c, _) => c.edges(det, s, t, node_factory),
            Command::Break => todo!(),
            Command::Continue => todo!(),
            Command::ModelCheckingArgs(_) => panic!("Model checking arguments should not be encountered at this stage"),
            Command::Parallel(_) => panic!("Parallel command type should not be encountered at this stage"),
        }
    }
}

impl AtomicStatement {
    fn edges(&self, det: Determinism, s: Node, t: Node) -> Vec<Edge> {
        match self {
            AtomicStatement::SimpleCommands(commands) => commands.edges(s, t),
            AtomicStatement::AtomicGuards(guards) => guards.edges(det, s, t),
        }
    }
}

impl SimpleCommands {
    fn edges(&self, s: Node, t: Node) -> Vec<Edge> {
        vec![Edge(s, Action::Atomic(self.clone()), t)]
    }
}

impl AtomicGuards {
    fn edges(&self, det: Determinism, s: Node, t: Node) -> Vec<Edge> {
        match det {
            Determinism::Deterministic => {
                let mut prev = BExpr::Bool(false);
    
                let mut edges = vec![];
    
                for AtomicGuard(b, c) in &self.0 {
                    edges.push(Edge(
                        s,
                        Action::ConditionalAtomic(
                            BExpr::logic(
                                b.clone(), 
                                LogicOp::Land, 
                                BExpr::Not(Box::new(prev.clone()))
                            ),
                            c.clone()
                        ),
                        t
                    ));

                    prev = BExpr::logic(b.to_owned().clone(), LogicOp::Lor, prev);
                }
    
                edges
            }
            Determinism::NonDeterministic => {
                self.0.iter()
                    .map(|AtomicGuard(bexp, commands)| Edge(s, Action::ConditionalAtomic(bexp.clone(), commands.clone()), t))
                    .collect()
            }
        }
    }
}

fn done(guards: &[Guard]) -> BExpr {
    guards
        .iter()
        .map(|Guard(b, _c)| BExpr::Not(Box::new(b.clone())))
        .reduce(|a, b| BExpr::logic(a, LogicOp::Land, b))
        .unwrap_or(BExpr::Bool(true))
}

impl ProgramGraph {
    pub fn new(det: Determinism, cmds: &Commands) -> Self {
        match &cmds.0[0] {
            Command::ModelCheckingArgs(_) => ProgramGraph::new(det, &Commands(cmds.0[1..].iter().map(Clone::clone).collect())),
            Command::Parallel(pcmds) => {
                ParallelProgramGraph::new(det, pcmds).to_pg()
            }
            _ => {
                let mut node_factory = NodeFactory::new();
                let edges = cmds.edges(det, Node::Start, Node::End, &mut node_factory);
                let mut outgoing: HashMap<Node, Vec<Edge>> = HashMap::new();
                let mut nodes: HashSet<Node> = Default::default();
        
                for e in &edges {
                    outgoing.entry(e.0).or_default().push(e.clone());
                    nodes.insert(e.0);
                    nodes.insert(e.2);
                }
        
                Self {
                    outgoing,
                    edges,
                    nodes,
                }
                .rename_with_reverse_post_order()
            }
        }

    }
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }
    pub fn nodes(&self) -> &HashSet<Node> {
        &self.nodes
    }
    pub fn outgoing(&self, node: Node) -> &[Edge] {
        self.outgoing
            .get(&node)
            .map(|s| s.as_slice())
            .unwrap_or_default()
    }

    pub fn fv(&self) -> HashSet<Target> {
        self.edges.iter().flat_map(|e| e.action().fv()).collect()
    }

    pub fn dot(&self) -> String {
        format!(
            "digraph G {{\n{}\n}}",
            self.edges
                .iter()
                .map(|e| format!(
                    "  {:?}[label=\"{}\"]; {:?} -> {:?}[label={:?}]; {:?}[label=\"{}\"];",
                    e.0,
                    e.0,
                    e.0,
                    e.2,
                    e.1.to_string(),
                    e.2,
                    e.2,
                ))
                .format("  \n")
        )
    }

    pub fn as_petgraph(
        &self,
    ) -> (
        petgraph::Graph<Node, Action>,
        BTreeMap<Node, petgraph::graph::NodeIndex>,
        BTreeMap<petgraph::graph::NodeIndex, Node>,
    ) {
        let mut g = petgraph::Graph::new();

        let node_mapping: BTreeMap<Node, petgraph::graph::NodeIndex> = self
            .nodes
            .iter()
            .copied()
            .map(|n| (n, g.add_node(n)))
            .collect();
        let node_mapping_rev: BTreeMap<petgraph::graph::NodeIndex, Node> =
            node_mapping.iter().map(|(a, b)| (*b, *a)).collect();

        for Edge(from, action, to) in &self.edges {
            g.add_edge(node_mapping[from], node_mapping[to], action.clone());
        }

        (g, node_mapping, node_mapping_rev)
    }

    pub fn rename_with_reverse_post_order(&self) -> Self {
        let (g, node_mapping, node_mapping_rev) = self.as_petgraph();

        let initial_node = if let Some(n) = node_mapping.get(&Node::Start) {
            *n
        } else {
            warn!("graph did not have a start node");
            return self.clone();
        };
        let mut dfs = petgraph::visit::DfsPostOrder::new(&g, initial_node);

        let mut new_order = VecDeque::new();

        while let Some(n) = dfs.next(&g) {
            new_order.push_front(node_mapping_rev[&n]);
        }

        let mut node_mapping_new: BTreeMap<Node, Node> = Default::default();

        enum NamingStage {
            Start,
            Middle { idx: u64 },
        }

        let mut stage = NamingStage::Start;
        for n in new_order.iter() {
            stage = match stage {
                NamingStage::Start => {
                    node_mapping_new.insert(*n, Node::Start);
                    NamingStage::Middle { idx: 1 }
                }
                NamingStage::Middle { idx } => match n {
                    Node::Start => todo!(),
                    Node::Node(_) => {
                        node_mapping_new.insert(*n, Node::Node(NodeId(idx)));
                        NamingStage::Middle { idx: idx + 1 }
                    }
                    Node::End => {
                        node_mapping_new.insert(*n, Node::End);
                        NamingStage::Middle { idx }
                    }
                    Node::ParallelStart(_)
                    | Node::ParallelNode(_, _)
                    | Node::ParallelEnd(_) => panic!("Parallel nodes should not appear at this stage")
                },
            }
        }

        Self {
            edges: self
                .edges
                .iter()
                .map(|Edge(a, action, b)| {
                    Edge(node_mapping_new[a], action.clone(), node_mapping_new[b])
                })
                .collect(),
            nodes: node_mapping_new.values().copied().collect(),
            outgoing: self
                .outgoing
                .iter()
                .map(|(n, outgoing)| {
                    (
                        node_mapping_new[n],
                        outgoing
                            .iter()
                            .map(|Edge(a, action, b)| {
                                Edge(node_mapping_new[a], action.clone(), node_mapping_new[b])
                            })
                            .collect(),
                    )
                })
                .collect(),
        }
    }
}
