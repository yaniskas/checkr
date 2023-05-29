use std::{collections::{BTreeSet, BTreeMap}, rc::Rc, fmt::Display};

use itertools::Itertools;

use crate::{ast::BExpr, util::traits::{AddMany, Singleton, Add}};

use super::ltl_ast::{NegativeNormalLTL, temporal_subformulae, until_subformulae};

#[derive(Debug, PartialEq, Eq)]
pub struct VWAA {
    pub states: BTreeSet<NegativeNormalLTL>,
    pub delta: BTreeMap<NegativeNormalLTL, BTreeSet<VWAATransitionResult>>,
    pub initial_states: BTreeSet<NegativeNormalLTL>,
    pub final_states: BTreeSet<NegativeNormalLTL>
}

impl VWAA {
    // Fast pg. 58 Step 1
    pub fn from_ltl(formula: &NegativeNormalLTL) -> VWAA {
        let states = temporal_subformulae(formula);
        let mut delta = BTreeMap::new();
        for state in &states {
            delta.insert(state.clone(), find_delta(state));
        }

        let initial_states = bar(formula);
        let final_states = until_subformulae(formula);

        VWAA {states, delta, initial_states, final_states}
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LTLConjunction {
    elements: BTreeSet<NegativeNormalLTL>,
}

impl LTLConjunction {
    pub fn tt() -> LTLConjunction {
        LTLConjunction {elements: BTreeSet::new()}
    }

    pub fn new(elements: BTreeSet<NegativeNormalLTL>) -> LTLConjunction {
        LTLConjunction {elements: elements.into_iter().filter(|e| *e != NegativeNormalLTL::True).collect()}
    }

    pub fn get_components(&self) -> BTreeSet<NegativeNormalLTL> {
        self.elements.clone().add(NegativeNormalLTL::True)
    }

    pub fn get_raw_components(&self) -> BTreeSet<NegativeNormalLTL> {
        self.elements.clone()
    }

    pub fn is_true(&self) -> bool {
        self.elements.is_empty()
    }
}

impl Conjuct for LTLConjunction {
    fn conjuct(&self, other: &LTLConjunction) -> LTLConjunction {
        LTLConjunction::new(self.get_components().add_many(other.get_components()))
    }
}

impl Display for LTLConjunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_components().into_iter().join(" && "))
    }
}

pub type VWAATransitionResult = (SymbolConjunction, LTLConjunction);

// Fast pg. 58 Definition 4
pub fn circle_x(j1: &BTreeSet<VWAATransitionResult>, j2: &BTreeSet<VWAATransitionResult>) -> BTreeSet<VWAATransitionResult> {
    let mut res = BTreeSet::new();
    for (alpha1, e1) in j1 {
        for (alpha2, e2) in j2 {
            if *alpha1 == SymbolConjunction::TT && *alpha2 == SymbolConjunction::TT {
                // println!("Inserting {:?}", (alpha1.conjuct(alpha2), e1.conjuct(e2)));
            }
            res.insert((alpha1.conjuct(alpha2), e1.conjuct(e2)));
        }
    }
    res
}

// Fast pg. 58 Definition 4
fn bar(formula: &NegativeNormalLTL) -> BTreeSet<NegativeNormalLTL> {
    let mut res = BTreeSet::new();
    match formula {
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
        _ => {
            res.insert(formula.clone());
        }
    }
    res
}

// Fast pg. 58 Step 1 definition of delta
fn find_delta(formula: &NegativeNormalLTL) -> BTreeSet<VWAATransitionResult> {
    let mut res = BTreeSet::new();
    match formula {
        NegativeNormalLTL::True => {
            res.insert((SymbolConjunction::TT, LTLConjunction::tt()));
        }
        NegativeNormalLTL::False => {}
        NegativeNormalLTL::Atomic(p) => {
            res.insert((SymbolConjunction::Conjunction(BTreeSet::singleton(Symbol::Atomic(p.clone()))), LTLConjunction::tt()));
        }
        NegativeNormalLTL::NegAtomic(p) => {
            res.insert((SymbolConjunction::Conjunction(BTreeSet::singleton(Symbol::NegAtomic(p.clone()))), LTLConjunction::tt()));
        }
        NegativeNormalLTL::Next(f) => {
            res.extend(bar(f).into_iter().map(|e| (SymbolConjunction::TT, LTLConjunction::new(BTreeSet::singleton(e)))));
        }
        NegativeNormalLTL::Until(f1, f2) => {
            res.extend(find_delta(f2));
            let mut operand = BTreeSet::new();
            operand.insert((SymbolConjunction::TT, LTLConjunction::new(BTreeSet::singleton(NegativeNormalLTL::Until(Rc::clone(f1), Rc::clone(f2))))));
            res.extend(circle_x(&find_delta(f1), &operand));
        }
        NegativeNormalLTL::Release(f1, f2) => {
            let mut union = BTreeSet::new();
            union.extend(find_delta(f1));
            union.insert((SymbolConjunction::TT, LTLConjunction::new(BTreeSet::singleton(NegativeNormalLTL::Release(Rc::clone(f1), Rc::clone(f2))))));
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