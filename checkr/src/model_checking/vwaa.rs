use std::{collections::{HashSet, HashMap}, rc::Rc, fmt::Display, hash::Hash};

use crate::ast::BExpr;

use super::ltl::{NegativeNormalLTL, temporal_subformulae, until_subformulae};

#[derive(Debug, PartialEq, Eq)]
pub struct VWAA {
    states: HashSet<NegativeNormalLTL>,
    delta: HashMap<NegativeNormalLTL, HashSet<VWAATransitionResult>>,
    initial_states: HashSet<NegativeNormalLTL>,
    final_states: HashSet<NegativeNormalLTL>
}

impl VWAA {
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

    pub fn delta(&self) -> &HashMap<NegativeNormalLTL, HashSet<VWAATransitionResult>> { &self.delta }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

trait Conjuct {
    fn conjuct(&self, other: &Self) -> Self;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SymbolConjunction {
    TT,
    Conjunction(Vec<Symbol>)
}

impl Conjuct for SymbolConjunction {
    fn conjuct(&self, other: &SymbolConjunction) -> SymbolConjunction {
        match self {
            SymbolConjunction::TT => other.clone(),
            SymbolConjunction::Conjunction(vec) => {
                let mut res: Vec<Symbol> = Vec::new();
                res.extend(vec.clone().into_iter());
                match other {
                    SymbolConjunction::TT => {},
                    SymbolConjunction::Conjunction(vec2) => res.extend(vec2.clone().into_iter())
                }
                SymbolConjunction::Conjunction(res)
            }
        }
    }
}

impl Conjuct for Vec<NegativeNormalLTL> {
    fn conjuct(&self, other: &Self) -> Self {
        match &self[..] {
            [NegativeNormalLTL::True] => other.clone(),
            _ => {
                match &other[..] {
                    [NegativeNormalLTL::True] => self.clone(),
                    _ => self.clone().add_many(other.clone())
                }
            }
        }
    }
}

impl Display for SymbolConjunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolConjunction::TT => write!(f, "tt"),
            SymbolConjunction::Conjunction(vec) => {
                let str = vec.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(" && ");
                write!(f, "{str}")
            }
        }
    }
}

type LTLConjunction = Vec<NegativeNormalLTL>;
type VWAATransitionResult = (SymbolConjunction, LTLConjunction);

fn circle_x(j1: &HashSet<VWAATransitionResult>, j2: &HashSet<VWAATransitionResult>) -> HashSet<VWAATransitionResult> {
    let mut res = HashSet::new();
    for (alpha1, e1) in j1 {
        for (alpha2, e2) in j2 {
            res.insert((alpha1.conjuct(alpha2), e1.conjuct(e2)));
        }
    }
    res
}

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
                    res.insert(NegativeNormalLTL::And(Rc::new(e1.clone()), Rc::new(e2.clone())));
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

fn find_delta(formula: &NegativeNormalLTL) -> HashSet<VWAATransitionResult> {
    let mut res = HashSet::new();
    match formula {
        NegativeNormalLTL::True => {
            res.insert((SymbolConjunction::TT, vec! [NegativeNormalLTL::True]));
        }
        NegativeNormalLTL::False => {}
        NegativeNormalLTL::Atomic(p) => {
            res.insert((SymbolConjunction::Conjunction(vec! [Symbol::Atomic(p.clone())]), vec! [NegativeNormalLTL::True]));
        }
        NegativeNormalLTL::NegAtomic(p) => {
            res.insert((SymbolConjunction::Conjunction(vec! [Symbol::NegAtomic(p.clone())]), vec! [NegativeNormalLTL::True]));
        }
        NegativeNormalLTL::Next(f) => {
            res.extend(bar(f).into_iter().map(|e| (SymbolConjunction::TT, vec! [e])));
        }
        NegativeNormalLTL::Until(f1, f2) => {
            res.extend(find_delta(f2));
            let mut operand = HashSet::new();
            operand.insert((SymbolConjunction::TT, vec! [NegativeNormalLTL::Until(Rc::clone(f1), Rc::clone(f2))]));
            res.extend(circle_x(&find_delta(f1), &operand));
        }
        NegativeNormalLTL::Release(f1, f2) => {
            let mut union = HashSet::new();
            union.extend(find_delta(f1));
            union.insert((SymbolConjunction::TT, vec! [NegativeNormalLTL::Release(Rc::clone(f1), Rc::clone(f2))]));
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

trait Add {
    type Item;
    fn add(self, item: Self::Item) -> Self;
}

trait AddMany {
    type Item;
    fn add_many(self, items: impl IntoIterator<Item = Self::Item>) -> Self;
}

impl <T> Add for HashSet<T> 
where
    T: Eq + Hash {
    type Item = T;

    fn add(mut self, item: Self::Item) -> Self {
        self.insert(item);
        self
    }
}

impl <T> AddMany for HashSet<T> 
where
    T: Eq + Hash {
    type Item = T;

    fn add_many(mut self, items: impl IntoIterator<Item = Self::Item>) -> Self {
        self.extend(items);
        self
    }
}

impl <T> Add for Vec<T> {
    type Item = T;

    fn add(mut self, item: Self::Item) -> Self {
        self.push(item);
        self
    }
}

impl <T> AddMany for Vec<T> {
    type Item = T;

    fn add_many(mut self, item: impl IntoIterator<Item = Self::Item>) -> Self {
        self.extend(item);
        self
    }
}