use std::cmp::Ordering;
use std::fmt::Display;
use std::hash::Hash;
use std::collections::{HashSet, HashMap, BTreeSet, VecDeque};
use std::rc::Rc;
use std::iter;

use crate::model_checking::vwaa::LTLConjunction;

use super::ltl::NegativeNormalLTL;
use super::vwaa::{Symbol, VWAATransitionResult, SymbolConjunction, VWAA, circle_x, Conjuct};
use super::traits::*;

type GBATransitionResult = (SymbolConjunction, LTLConjunction);

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub struct GBATransition(pub LTLConjunction, pub SymbolConjunction, pub LTLConjunction);

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GBA {
    pub states: HashSet<LTLConjunction>,
    pub delta: HashMap<LTLConjunction, HashSet<GBATransitionResult>>,
    pub initial_state: LTLConjunction,
    pub accepting_transitions: BTreeSet<BTreeSet<GBATransition>>,
}

pub fn ltlset_string(ltlset: &LTLConjunction) -> String {
    match ltlset {
        LTLConjunction::TT => "tt".to_string(),
        LTLConjunction::Conjunction(ltlset) => ltlset.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(" && ")
    }
}

impl GBA {
    pub fn from_vwaa(vwaa: VWAA) -> GBA {
        let VWAA { states, delta, initial_states, final_states } = vwaa;
        
        println!("Subset: {}", initial_states.is_subset(&states));
        let q_prime = ltl_power_set(states);

        println!("Original number of states: {}", q_prime.len());

        let delta2prime = q_prime.iter()
            .map(|ltlcon| {
                let results = match ltlcon {
                    LTLConjunction::TT => delta[&NegativeNormalLTL::True].clone().into_iter().collect::<BTreeSet<_>>(),
                    LTLConjunction::Conjunction(set) => set.iter()
                        .map(|e| delta[e].clone())
                        .reduce(|e1, e2| circle_x(&e1, &e2))
                        .unwrap_or(HashSet::new().add((SymbolConjunction::TT, LTLConjunction::Conjunction(BTreeSet::new().add(NegativeNormalLTL::False)))))
                        .into_iter()
                        .collect::<BTreeSet<_>>()
                };
                (ltlcon.clone(), results)
            })
            .collect::<HashMap<_, _>>();

        let initial_state = LTLConjunction::Conjunction(initial_states.into_iter().collect::<BTreeSet<_>>());
        println!("Initial state: {}", ltlset_string(&initial_state));
        println!("In Q_prime: {}", q_prime.contains(&initial_state));
        
        println!("Number of transition results before removing non-reachable: {}", delta2prime.len());
        let delta2prime = get_reachable(delta2prime, &initial_state);
        println!("Number of transition results after removing non-reachable: {}", delta2prime.len());
        println!("{:?}", delta2prime);

        let delta2primetransitions = delta2prime
            .clone()
            .into_iter()
            .flat_map(|(source, set)| {
                set.into_iter().map(move |(action, sink)| GBATransition(source.clone(), action, sink))
            })
            .collect::<HashSet<_>>();

        let accepting_transitions = find_accepting_transitions(&final_states, &delta, &delta2prime);
        println!("Number of accepting transition sets: {}", accepting_transitions.len());
        for ats in &accepting_transitions {
            println!("Number of accepting transitions: {}", ats.len());
            println!("Accepting transitions in this set:");
            for at in ats {
                print!("Transition: ");
                let GBATransition(source, action, target) = at;
                println!("source: {} action: {} target: {}", ltlset_string(source), action, ltlset_string(target));
            }
        }

        println!("Number of transitions before reduction: {}", delta2primetransitions.len());
        let deltaprimetransitions = remove_non_minimal(delta2primetransitions, transition_comparator(&accepting_transitions));
        // let deltaprimetransitions = delta2primetransitions;
        println!("Number of transitions after reduction: {}", deltaprimetransitions.len());

        let accepting_transitions = accepting_transitions.into_iter()
            .map(|ats| {
                ats.into_iter()
                    .filter(|t| deltaprimetransitions.contains(t))
                    .collect::<BTreeSet<_>>()
            })
            .collect::<BTreeSet<_>>();
        println!("Number of accepting transition sets: {}", accepting_transitions.len());
        for ats in &accepting_transitions {
            println!("Number of accepting transitions: {}", ats.len());
        }

        let delta_prime = agglomerate_transitions(deltaprimetransitions);
        
        println!("Delta prime:");
        println!("{}", delta_prime.contains_key(&initial_state));

        let Q_prime = delta_prime.iter()
            .map(|(source, targets)| source.clone())
            .collect::<HashSet<_>>();

        GBA {
            states: Q_prime,
            delta: delta_prime,
            initial_state,
            accepting_transitions
        }

    }

    pub fn get_next_edges(&self, state: &LTLConjunction) -> HashSet<&GBATransitionResult> {
        match self.delta.get(&state) {
            Some(results) => results.iter().collect(),
            None => HashSet::new()
        }
    }
}

pub fn agglomerate_transitions(transitions: HashSet<GBATransition>) -> HashMap<LTLConjunction, HashSet<GBATransitionResult>> {
    let delta_prime = transitions.into_iter()
        .fold(HashMap::new(), |mut acc, GBATransition(source, action, sink)| {
            acc.entry(source).or_insert(HashSet::new()).insert((action, sink));
            acc
        });
    delta_prime
}

pub fn states_from_transitions(transitions: &HashSet<GBATransition>) -> HashSet<LTLConjunction> {
    transitions.iter()
            .flat_map(|GBATransition(source, symcon, target)| [source.clone(), target.clone()])
            .collect()
}

fn transition_comparator<'a>(accepting_transitions: &'a BTreeSet<BTreeSet<GBATransition>>) -> impl Fn(&GBATransition, &GBATransition) -> Option<Ordering> + 'a {
    |first: &GBATransition, second: &GBATransition| {
        if first.0 != second.0 {return None};

        if transition_less(first, second, accepting_transitions) {Some(Ordering::Less)}
        else if transition_less(second, first, accepting_transitions) {Some(Ordering::Greater)}
        else {None}
    }
}

fn transition_less(first: &GBATransition, second: &GBATransition, accepting_transitions: &BTreeSet<BTreeSet<GBATransition>>) -> bool {
    let GBATransition(_, alpha, eprime) = second;
    let GBATransition(_, alphaprime, e2prime) = first;

    alpha.is_subset(alphaprime) 
        && e2prime.is_subset(eprime) 
        && accepting_transitions.iter().all(|trans_set| {
            !trans_set.contains(second) || trans_set.contains(first)
        })
}

fn ltl_power_set(set: HashSet<NegativeNormalLTL>) -> HashSet<LTLConjunction> {
    let rc_set = set.into_iter().filter(|e| e != &NegativeNormalLTL::True && e != &NegativeNormalLTL::False)
        .map(Rc::new).collect::<HashSet<_>>();

    rc_set.into_iter().fold(HashSet::new().add(LTLConjunction::Conjunction(BTreeSet::new())), |acc, elem| {
        acc.clone().add_many(
            acc
                .into_iter()
                .map(|e| e.conjuct(&LTLConjunction::Conjunction(BTreeSet::new().add((*elem).clone()))))
        )
    }).add(LTLConjunction::TT)
    // TODO: Figure out how to add the true in a nicer way
}

fn remove_non_minimal<T: Eq + Hash>(set: HashSet<T>, comparator: impl Fn(&T, &T) -> Option<Ordering>) -> HashSet<T> {
    set.into_iter().fold(HashSet::new(), |acc: HashSet<T>, new_elem| {
        let original_size = acc.len();
        let new_set = acc.into_iter().filter(|elem| comparator(elem, &new_elem) != Some(Ordering::Greater)).collect::<HashSet<T>>();
        let new_size = new_set.len();

        if new_size < original_size 
            || new_set.iter().all(|elem| comparator(&new_elem, elem) != Some(Ordering::Greater))
            {new_set.add(new_elem)}
        else {new_set}
    })
}

fn get_reachable(delta: HashMap<LTLConjunction, BTreeSet<GBATransitionResult>>, initial: &LTLConjunction) -> HashMap<LTLConjunction, BTreeSet<GBATransitionResult>> {
    let vec = delta.into_iter().collect::<Vec<_>>();
    let state_to_index = vec.iter()
        .enumerate()
        .map(|(i, (state, _))| {
            (state.clone(), i)
        })
        .collect::<HashMap<_, _>>();

    let mut visited = HashSet::new();

    let empty_set = BTreeSet::new();

    let mut queue = VecDeque::new();
    queue.push_back(initial);
    while let Some(current_state) = queue.pop_front() {
        println!("Current state: {}", ltlset_string(current_state));

        if (visited.contains(current_state)) {
            println!("State already visited");
            continue;
        }

        let trans_result = match state_to_index.get(current_state) {
            Some(index) => {
                let (_, trans_result) = &vec[*index];
                trans_result
            }
            None => &empty_set
        };

        let next_states = trans_result.iter()
            .map(|(symcon, target)| target);

        queue.extend(next_states);

        visited.insert(current_state.clone());
    }

    vec.into_iter()
        .filter(|(value, targets)| visited.contains(value))
        .collect::<HashMap<_, _>>()

}

pub trait Subsettable {
    fn is_subset(&self, other: &Self) -> bool;
}

impl Subsettable for SymbolConjunction {
    fn is_subset(&self, other: &Self) -> bool {
        match other {
            SymbolConjunction::TT => true,
            SymbolConjunction::Conjunction(otherset) => {
                match self {
                    SymbolConjunction::TT => false,
                    SymbolConjunction::Conjunction(selfset) => otherset.is_subset(selfset)
                }
            }
        }
    }
}

impl Subsettable for LTLConjunction {
    fn is_subset(&self, other: &Self) -> bool {
        match other {
            LTLConjunction::TT => *self == LTLConjunction::TT,
            LTLConjunction::Conjunction(otherset) => {
                match self {
                    LTLConjunction::TT => true,
                    // Uses the opposite logic here as compared to SymbolConjunction due to the different interpretation given to it
                    LTLConjunction::Conjunction(selfset) => selfset.is_subset(otherset)
                }
            }
        }
    }
}

trait Contains {
    type Item;
    fn contains(&self, item: &Self::Item) -> bool;
}

impl Contains for LTLConjunction {
    type Item = NegativeNormalLTL;

    fn contains(&self, item: &Self::Item) -> bool {
        match self {
            LTLConjunction::TT => false,
            LTLConjunction::Conjunction(set) => set.contains(item)
        }
    }
}

fn find_accepting_transitions(
    final_states: &HashSet<NegativeNormalLTL>, 
    delta: &HashMap<NegativeNormalLTL, HashSet<VWAATransitionResult>>,
    delta2prime: &HashMap<LTLConjunction, BTreeSet<GBATransitionResult>>
) -> BTreeSet<BTreeSet<GBATransition>> {
    final_states.iter()
        .map(|fstate| {
            let delta_fstate = delta[fstate].iter().collect::<Vec<_>>();

            delta2prime.iter()
                .flat_map(|(e, set)| {
                    set.iter().map(move |(alpha, eprime)| (e, alpha, eprime))
                })
                .filter(|(e, alpha, eprime)| {
                    !eprime.contains(fstate) || delta_fstate.iter().any(|(beta, e2prime)| {
                        alpha.is_subset(&beta) && !e2prime.contains(fstate) && e2prime.is_subset(eprime)
                    })
                })
                .map(|(e, alpha, eprime)| GBATransition(e.clone(), alpha.clone(), eprime.clone()))
                .collect()
        })
        .collect()
}