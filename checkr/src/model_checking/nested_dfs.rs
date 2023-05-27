use std::collections::{BTreeSet, VecDeque};

use crate::{model_checking::{vwaa::{SymbolConjunction, Symbol}}, ast::{BExpr, LogicOp}, pg::{Action}, concurrency::{ParallelProgramGraph, ParallelConfiguration, next_configurations}, util::traits::AddMany};

use super::{ba::{BA, BAState}, ModelCheckMemory};

pub struct ProductTransitionSystem<'a> {
    pub program_graph: &'a ParallelProgramGraph,
    buchi: &'a BA,
}

pub type ProductNode = (ParallelConfiguration, TrappingBAState);

impl <'a> ProductTransitionSystem<'a> {
    pub fn new(program_graph: &'a ParallelProgramGraph, buchi: &'a BA) -> Self {
        Self { program_graph, buchi }
    }

    pub fn next_nodes(&self, node: &ProductNode) -> impl IntoIterator<Item = (Action, ProductNode)> + 'a {
        let (config, tbastate) = node;

        if let TrappingBAState::NormalState(bastate) = tbastate {
            // println!("In normal state");
            let potential_next_configs = next_configurations(self.program_graph, &config);

            //
            let potential_next_configs = if potential_next_configs.len() == 0 {
                vec![(Action::Skip, config.clone())]
            } else {
                potential_next_configs
            };
            //

            // let potential_next_ba_states = dbg!(self.buchi.get_next_edges(dbg!(bastate)));
            let potential_next_ba_states = self.buchi.get_next_edges(bastate);

            let next_nodes = potential_next_configs.into_iter()
                .flat_map(move |(action, config)| {
                    potential_next_ba_states.iter()
                        .filter_map(|(symcon, bastate)| {
                            let condition = symcon_to_bexp(symcon);
                            if condition.semantics(&config.memory) == Ok(true) {
                                // println!("BExp {} is true in memory {:?}", condition, config.memory);
                                // println!("Leading to (config, bastate) = ({:?}, {:?})", config, bastate);
                                Some((action.clone(), (config.clone(), bastate.clone())))
                            }
                            else {
                                // println!("BExp {} is NOT true in memory {:?}", condition, config.memory);
                                // println!("Leading to (config, bastate) = ({:?}, {:?})", config, bastate);
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .map(|(action, (config, bastate))| (action, (config, TrappingBAState::NormalState(bastate))))
                .collect::<Vec<_>>();
            
            if next_nodes.len() != 0 {
                // println!("Returning normal states");
                next_nodes
            } else {
                // If there are no valid edges, keep the same transition system node and make the BA move to a trap state
                // Principles pg. 187
                // println!("Returning trap state");
                vec![(Action::Skip, (config.clone(), TrappingBAState::TrapState))]
            }
        } else {
            // If the BA is in the trap state, do not change state
            // println!("In trap state, returning trap state");
            vec![(Action::Skip, node.clone())]
        }
    }

    pub fn initial_nodes(&self, initial_memory: &'a ModelCheckMemory) -> impl Iterator<Item = ProductNode> + 'a {
        self.buchi.get_next_edges(&self.buchi.initial_state).into_iter()
            .filter(|(action, _bastate)| {
                let condition = symcon_to_bexp(action);
                condition.semantics(initial_memory) == Ok(true)
            })
            .map(|(_action, bastate)| {
                (ParallelConfiguration {nodes: self.program_graph.initial_nodes(), memory: initial_memory.clone()}, TrappingBAState::NormalState(bastate.clone()))
            })
    }

    pub fn state_is_final(&self, state: &ProductNode) -> bool {
        let bastate = &state.1;
        match bastate {
            TrappingBAState::NormalState(state) => state.1 == self.buchi.top_layer,
            TrappingBAState::TrapState => false
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrappingBAState {
    NormalState(BAState),
    TrapState
}

pub type PathFragment = Vec<(Action, ProductNode)>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum LTLVerificationResult {
    CycleFound{trace: PathFragment, cycle_start: usize},
    CycleNotFound,
    SearchDepthExceeded,
}

fn symcon_to_bexp(symcon: &SymbolConjunction) -> BExpr {
    match symcon {
        SymbolConjunction::TT => BExpr::Bool(true),
        SymbolConjunction::Conjunction(symset) => {
            symset.into_iter()
            .map(|symbol| {
                match symbol {
                    Symbol::Atomic(bexp) => bexp.clone(),
                    Symbol::NegAtomic(bexp) => BExpr::Not(Box::new(bexp.clone())),
                }
                })
                .reduce(|acc, bexp| {
                    BExpr::Logic(Box::new(acc), LogicOp::And, Box::new(bexp))
                })
                .unwrap()
        }
    }
}

pub fn nested_dfs(program_graph: &ParallelProgramGraph, buchi: &BA, initial_memory: &ModelCheckMemory, search_depth: usize) -> LTLVerificationResult {
    let product = ProductTransitionSystem::new(program_graph, buchi);

    let mut r: BTreeSet<ProductNode> = BTreeSet::new();
    let mut t: BTreeSet<ProductNode> = BTreeSet::new();

    let mut search_depth_exceeded = false;
    
    for s in product.initial_nodes(initial_memory) {
        if !r.contains(&s) {
            match reachable_cycle(&s, &product, &mut r, &mut t, search_depth) {
                LTLVerificationResult::CycleNotFound => continue,
                LTLVerificationResult::SearchDepthExceeded => {
                    search_depth_exceeded = true;
                    continue;
                }
                trace => return trace
            }
        }
    }

    if search_depth_exceeded {LTLVerificationResult::SearchDepthExceeded} else {LTLVerificationResult::CycleNotFound}
}

fn reachable_cycle(s: &ProductNode, product: &ProductTransitionSystem, r: &mut BTreeSet<ProductNode>, t: &mut BTreeSet<ProductNode>, search_depth: usize) -> LTLVerificationResult {
    let mut u: VecDeque<(Action, ProductNode)> = VecDeque::new();

    u.push_front((Action::Skip, s.clone()));
    r.insert(s.clone());

    let mut search_depth_exceeded = false;

    while let Some(s_prime) = u.front() {
        if u.len() >= search_depth {
            search_depth_exceeded = true;
            u.pop_front();
            continue;
        }

        // println!("Iterating outer DFS");
        // println!("Checking state {:#?}", s_prime);
        // TODO
        let post_s_prime = product.next_nodes(&s_prime.1).into_iter().collect::<Vec<_>>();
        // println!("Found next nodes");
        // println!("Number of next nodes: {}", post_s_prime.len());
        match post_s_prime.into_iter().find(|(_act, e)| !r.contains(e)) {
            Some(s2prime) => {
                // println!("Found new node");
                u.push_front(s2prime.clone());
                r.insert(s2prime.1);
            },
            None => {
                // println!("Extracting s prime");
                let s_prime = u.pop_front().unwrap();
                // println!("Removed s prime");
                if product.state_is_final(&s_prime.1) {
                    // println!("State {:#?} is final", s_prime);
                    // println!("Calling inner DFS");
                    let cycle_found = cycle_check(&s_prime, product, t, search_depth);
                    match cycle_found {
                        LTLVerificationResult::CycleFound{trace: v, cycle_start: _} => {
                            let u: Vec<_> = u.into_iter().rev().collect();
                            let u_len = u.len();
                            let trace = u.add_many(v);

                            // for i in 0..(trace.len() - 1) {
                            //     if !product.next_nodes(&trace[i].1).into_iter().collect::<Vec<_>>().contains(&trace[i+1]) {
                            //         panic!("Product discontinuity: {:#?}, {:#?}", &trace[i], &trace[i+1])
                            //     }
                            // }

                            // for i in 0..(trace.len() - 1) {
                            //     let pc = &trace[i].1.0;
                            //     let next_configs = next_configurations(product.program_graph, pc).into_iter()
                            //         .map(|(_, config)| config)
                            //         .collect::<Vec<_>>();
                            //     if !next_configs.contains(&trace[i+1].1.0) {
                            //         panic!("PG discontinuity: {:#?}, {:#?}", &trace[i].0, &trace[i+1].0)
                            //     }
                            // }

                            return LTLVerificationResult::CycleFound{trace, cycle_start: u_len};
                        }
                        LTLVerificationResult::CycleNotFound => continue,
                        LTLVerificationResult::SearchDepthExceeded => {
                            search_depth_exceeded = true;
                            continue;
                        }
                    }
                } else {
                    // println!("State {:?} is NOT final", s_prime);
                }
            }
        }
    }

    if search_depth_exceeded {LTLVerificationResult::SearchDepthExceeded} else {LTLVerificationResult::CycleNotFound}
}

fn cycle_check(s: &(Action, ProductNode), product: &ProductTransitionSystem, t: &mut BTreeSet<ProductNode>, search_depth: usize) -> LTLVerificationResult {
    let mut v: VecDeque<(Action, ProductNode)> = VecDeque::new();

    v.push_front(s.clone());
    t.insert(s.1.clone());

    let mut search_depth_exceeded = false;

    while let Some(s_prime) = v.front() {
        if v.len() >= search_depth {
            search_depth_exceeded = true;
            v.pop_front();
            continue;
        }

        // println!("Iterating inner DFS");
        let post_s_prime = product.next_nodes(&s_prime.1).into_iter().collect::<BTreeSet<_>>();
        if post_s_prime.iter().map(|(_action, config)| config).collect::<Vec<_>>().contains(&&s.1) {
            v.push_front(s.clone());
            // println!("Found cycle to final state {:#?}", s);
            return LTLVerificationResult::CycleFound{trace: v.into_iter().rev().collect(), cycle_start: 0};
        } else {
            if let Some(s2prime) = post_s_prime.iter().find(|(_action, e)| !t.contains(e)) {
                v.push_front(s2prime.clone());
                t.insert(s2prime.1.clone());
            } else {
                v.pop_front();
            }
        }
    }
    
    if search_depth_exceeded {LTLVerificationResult::SearchDepthExceeded} else {LTLVerificationResult::CycleNotFound}
}