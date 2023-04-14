use std::{collections::{HashSet, HashMap, BTreeSet}, rc::Rc, fmt::Display, hash::Hash};

use crate::ast::BExpr;

use super::ltl::{NegativeNormalLTL, temporal_subformulae, until_subformulae};
use super::traits::*;

#[derive(Debug, PartialEq, Eq)]
pub struct VWAA {
    pub states: HashSet<NegativeNormalLTL>,
    pub delta: HashMap<NegativeNormalLTL, HashSet<VWAATransitionResult>>,
    pub initial_states: HashSet<NegativeNormalLTL>,
    pub final_states: HashSet<NegativeNormalLTL>
}

impl VWAA {
    // Fast pg. 58 Step 1
    pub fn from_ltl(formula: &NegativeNormalLTL) -> VWAA {
        let states = temporal_subformulae(formula);
        let mut delta = HashMap::new();
        for state in &states {
            delta.insert(state.clone(), find_delta(state));
        }
        let initial_states = bar(formula);
        let final_states = until_subformulae(formula);

        VWAA {states, delta, initial_states, final_states}
    }

    pub fn delta(&self) -> &HashMap<NegativeNormalLTL, HashSet<VWAATransitionResult>> {&self.delta}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Symbol {
    Atomic(BExpr),
    NegAtomic(BExpr)
}

impl Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Symbol::Atomic(b) => write!(f, "{b}"),
            Symbol::NegAtomic(b) => write!(f, "!{b}")
        }
    }
}

// Used in the circle_x operator
pub trait Conjuct {
    fn conjuct(&self, other: &Self) -> Self;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SymbolConjunction {
    TT,
    Conjunction(BTreeSet<Symbol>)
}

impl Conjuct for SymbolConjunction {
    fn conjuct(&self, other: &SymbolConjunction) -> SymbolConjunction {
        match self {
            SymbolConjunction::TT => other.clone(),
            SymbolConjunction::Conjunction(set) => {
                match other {
                    SymbolConjunction::TT => SymbolConjunction::Conjunction(set.clone()),
                    SymbolConjunction::Conjunction(set2) => SymbolConjunction::Conjunction(set.clone().add_many(set2.clone()))
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LTLConjunction {
    TT,
    Conjunction(BTreeSet<NegativeNormalLTL>),
}

impl Conjuct for LTLConjunction {
    fn conjuct(&self, other: &LTLConjunction) -> LTLConjunction {
        match self {
            LTLConjunction::TT => other.clone(),
            LTLConjunction::Conjunction(set) => {
                match other {
                    LTLConjunction::TT => LTLConjunction::Conjunction(set.clone()),
                    LTLConjunction::Conjunction(set2) => LTLConjunction::Conjunction(set.clone().add_many(set2.clone()))
                }
            }
        }
    }
}

impl Display for SymbolConjunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolConjunction::TT => write!(f, "tt"),
            SymbolConjunction::Conjunction(set) => {
                let str = set.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(" && ");
                write!(f, "{str}")
            }
        }
    }
}

pub type VWAATransitionResult = (SymbolConjunction, LTLConjunction);

// Fast pg. 58 Definition 4
pub fn circle_x(j1: &HashSet<VWAATransitionResult>, j2: &HashSet<VWAATransitionResult>) -> HashSet<VWAATransitionResult> {
    let mut res = HashSet::new();
    for (alpha1, e1) in j1 {
        for (alpha2, e2) in j2 {
            if *alpha1 == SymbolConjunction::TT && *alpha2 == SymbolConjunction::TT {
                println!("Inserting {:?}", (alpha1.conjuct(alpha2), e1.conjuct(e2)));
            }
            res.insert((alpha1.conjuct(alpha2), e1.conjuct(e2)));
        }
    }
    res
}

// Fast pg. 58 Definition 4
fn bar(formula: &NegativeNormalLTL) -> HashSet<NegativeNormalLTL> {
    let mut res = HashSet::new();
    match formula {
        NegativeNormalLTL::Next(_) | NegativeNormalLTL::Until(_, _) | NegativeNormalLTL::Release(_, _) => {
            res.insert(formula.clone());
        }
        NegativeNormalLTL::And(f1, f2) => {
            let barf2 = bar(f2);
            for e1 in bar(f1) {
                for e2 in &barf2 {
                    // ?
                    res.insert(e1.clone());
                    res.insert(e2.clone());
                    // res.insert(NegativeNormalLTL::And(Rc::new(e1.clone()), Rc::new(e2.clone())));
                }
            }
        }
        NegativeNormalLTL::Or(f1, f2) => {
            res.extend(bar(f1));
            res.extend(bar(f2));
        }
        _ => {}
    }
    res
}

// Fast pg. 58 Step 1 definition of delta
fn find_delta(formula: &NegativeNormalLTL) -> HashSet<VWAATransitionResult> {
    let mut res = HashSet::new();
    match formula {
        NegativeNormalLTL::True => {
            res.insert((SymbolConjunction::TT, LTLConjunction::TT));
        }
        NegativeNormalLTL::False => {}
        NegativeNormalLTL::Atomic(p) => {
            res.insert((SymbolConjunction::Conjunction(BTreeSet::singleton(Symbol::Atomic(p.clone()))), LTLConjunction::TT));
        }
        NegativeNormalLTL::NegAtomic(p) => {
            res.insert((SymbolConjunction::Conjunction(BTreeSet::singleton(Symbol::NegAtomic(p.clone()))), LTLConjunction::TT));
        }
        NegativeNormalLTL::Next(f) => {
            res.extend(bar(f).into_iter().map(|e| (SymbolConjunction::TT, LTLConjunction::Conjunction(BTreeSet::singleton(e)))));
        }
        NegativeNormalLTL::Until(f1, f2) => {
            res.extend(find_delta(f2));
            let mut operand = HashSet::new();
            operand.insert((SymbolConjunction::TT, LTLConjunction::Conjunction(BTreeSet::singleton(NegativeNormalLTL::Until(Rc::clone(f1), Rc::clone(f2))))));
            res.extend(circle_x(&find_delta(f1), &operand));
        }
        NegativeNormalLTL::Release(f1, f2) => {
            let mut union = HashSet::new();
            union.extend(find_delta(f1));
            union.insert((SymbolConjunction::TT, LTLConjunction::Conjunction(BTreeSet::singleton(NegativeNormalLTL::Release(Rc::clone(f1), Rc::clone(f2))))));
            res.extend(
                circle_x(&find_delta(f2), &union)
            )
        }
        NegativeNormalLTL::Or(f1, f2) => {
            res.extend(find_delta(f1));
            res.extend(find_delta(f2));
        }
        NegativeNormalLTL::And(f1, f2) => {
            res.extend(circle_x(&find_delta(f1), &find_delta(f2)));
        }
    }
    // res
    //     .into_iter()
    //     .flat_map(|e| 
    //         split_conjunctions(e.1).into_iter().map(move |s| (e.0.clone(), s))
    //     ).collect()

    res
}