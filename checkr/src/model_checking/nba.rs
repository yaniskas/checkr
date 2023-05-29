use std::{collections::{BTreeSet, BTreeMap}, fmt::Display};

use crate::util::traits::{Add, AddMany};

use super::{vwaa::{SymbolConjunction, LTLConjunction}, gba::{GBA, GBATransition}};

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
// state number, layer
pub struct NBAState(pub usize, pub usize);

impl Display for NBAState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, {}", self.0, self.1)
    }
}

pub type NBATransitionResult = (SymbolConjunction, NBAState);

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct NBA {
    pub delta: BTreeMap<NBAState, BTreeSet<NBATransitionResult>>,
    pub initial_state: NBAState,
    pub top_layer: usize,
}

impl NBA {
    pub fn from_gba(gba: GBA) -> NBA {
        // Fast pg. 62 step 3
        // GBA-NBA conversion can give different results depending on the order of accepting transition sets, but it seems 
        // that the results are equivalent
        let GBA {states, delta: _, initial_state, accepting_transitions} = &gba;

        let state_to_int = states.iter()
            .enumerate()
            .map(|(num, state)| (state.clone(), num))
            .collect::<BTreeMap<_, _>>();

        let accepting_transitions = accepting_transitions.clone().into_iter().collect::<Vec<_>>();
        let top_layer = accepting_transitions.len();

        let delta_prime = states.iter()
            .flat_map(|q| {
                (0..=top_layer).into_iter()
                    .map(|j| {
                        let targets = gba.get_next_edges(q).into_iter()
                            .map(|(alpha, q_prime)| {
                                let j_prime = next(j, q, alpha, q_prime, &accepting_transitions);
                                (alpha.clone(), NBAState(state_to_int[q_prime], j_prime))
                            })
                            .collect::<BTreeSet<_>>();
                        (NBAState(state_to_int[q], j), targets)
                    })
            })
            .collect();

        let initial_state_nba = NBAState(state_to_int[&initial_state], 0);

        NBA {delta: delta_prime, initial_state: initial_state_nba, top_layer}
    }

    pub fn get_states(&self) -> BTreeSet<NBAState> {
        self.delta.iter()
            .flat_map(|(source, targets)| {
                Vec::new()
                    .add(source.clone())
                    .add_many(targets.iter().map(|(_symcon, target)| target.clone()))
            })
            .collect()
    }

    pub fn get_next_edges(&self, state: &NBAState) -> BTreeSet<&NBATransitionResult> {
        match self.delta.get(&state) {
            Some(results) => results.iter().collect(),
            None => BTreeSet::new()
        }
    }
}

fn next(j: usize, q: &LTLConjunction, alpha: &SymbolConjunction, q_prime: &LTLConjunction, accepting_transtions: &Vec<BTreeSet<GBATransition>>) -> usize {
    let r = accepting_transtions.len();
    // let start_pos = if j == r {0} else {j} + 1;
    let start_pos = if j == r {0} else {j};
    let t = GBATransition(q.clone(), alpha.clone(), q_prime.clone());


    (start_pos..=r).into_iter().rev().find(|i| {
        ((start_pos+1)..=*i).into_iter()
            .all(|k| accepting_transtions[k-1].contains(&t))
    }).unwrap()
    // for i in start_pos..=r {
    //     if !accepting_transtions[i-1].contains(&t) {
    //         return i - 1;
    //     }
    // }
    // return r;
}