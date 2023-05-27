use std::collections::{BTreeSet, BTreeMap, VecDeque};
use std::fmt::Debug;

use crate::util::traits::{Add, WithRemoved, AddMany};

use super::ba::{BA, BAState};
use super::gba::{GBA, GBATransition, Subsettable, agglomerate_transitions};
use super::vwaa::{LTLConjunction, SymbolConjunction};

// Based on Fast pg. 63 / Section 6
pub trait SimplifiableAutomaton: Clone + Eq {
    type State: Ord + Eq + Clone + Debug;
    type Transition: Ord + Eq;

    fn simplify(self) -> Self {
        let mut automaton = self;
        loop {
            let old_automaton = automaton.clone();
            
            automaton = automaton.remove_inaccessible_states().remove_implied_transitions().remove_equivalent_states();

            if old_automaton == automaton {return automaton}
        }
    }

    fn get_initial_states(&self) -> BTreeSet<Self::State>;
    fn get_next_states(&self, state: &Self::State) -> BTreeSet<Self::State>;
    fn with_only_states(self, states: BTreeSet<Self::State>) -> Self;

    fn remove_inaccessible_states(self) -> Self {
        let initial_states = self.get_initial_states();

        let mut queue = VecDeque::new();
        queue.extend(initial_states);

        let mut visited = BTreeSet::new();

        while let Some(current_state) = queue.pop_front() {
            if visited.contains(&current_state) {
                continue;
            }

            queue.extend(self.get_next_states(&current_state));

            visited.insert(current_state);
        }

        self.with_only_states(visited)
    }

    fn get_transitions(&self) -> BTreeSet<Self::Transition>;
    fn transition_implies(&self, first: &Self::Transition, second: &Self::Transition) -> bool;
    fn with_only_transitions(self, transitions: BTreeSet<Self::Transition>) -> Self;

    fn remove_implied_transitions(self) -> Self {
        let transitions = self.get_transitions();

        let mut transitions_new = BTreeSet::new();

        for t in transitions {
            if transitions_new.iter()
                .all(|e| !self.transition_implies(e, &t)) {
                transitions_new = transitions_new.into_iter()
                    .filter(|e| !self.transition_implies(&t, e))
                    .collect();
                transitions_new.insert(t);
            }
        }

        self.with_only_transitions(transitions_new)
    }

    fn get_states(&self) -> BTreeSet<Self::State>;
    fn states_equivalent(&self, state1: &Self::State, state2: &Self::State) -> bool;
    fn with_states_merged(self, to_merge: Vec<(Self::State, Self::State)>) -> Self;

    fn remove_equivalent_states(self) -> Self {
        let state_vec = self.get_states().into_iter().collect::<Vec<_>>();

        let mut dealt_vec = Vec::new();
        for _ in 0..state_vec.len() {dealt_vec.push(false)};

        let mut to_merge = Vec::new();

        for i in 0..state_vec.len() {
            if dealt_vec[i] {continue};

            for j in i+1..state_vec.len() {
                if dealt_vec[j] {continue};

                let state1 = &state_vec[i];
                let state2 = &state_vec[j];
                if self.states_equivalent(state1, state2) {
                    // println!("The following states are equivalent:");
                    // println!("{:?}", state1);
                    // println!("{:?}", state2);
                    self.get_next_states(state1);
                    self.get_next_states(state2);
                    to_merge.push((state1.clone(), state2.clone()));
                    dealt_vec[j] = true;
                }
            }

            dealt_vec[i] = true;
        }

        self.with_states_merged(to_merge)
    }
}

impl SimplifiableAutomaton for GBA {
    type State = LTLConjunction;

    type Transition = GBATransition;

    fn get_initial_states(&self) -> BTreeSet<Self::State> {
        BTreeSet::new().add(self.initial_state.clone())
    }

    fn get_next_states(&self, state: &Self::State) -> BTreeSet<Self::State> {
        // println!("Results:");
        match self.delta.get(&state) {
            Some(results) => results.iter().map(|(_symcon, target)| {
                // println!("{}", symcon);
                // println!("{:?}", target);
                target.clone()
        }).collect(),
            None => BTreeSet::new()
        }
    }

    fn with_only_states(self, states: BTreeSet<Self::State>) -> Self {
        // println!("Creating GBA with only states {:?}", states);
        let GBA {states: _, delta, initial_state, accepting_transitions} = self;

        let delta = delta.into_iter()
            .filter(|(source, _targets)| states.contains(source))
            .map(|(source, targets)| {
                let targets = targets.into_iter()
                    .filter(|(_symcon, target)| states.contains(target))
                    .collect();
                (source, targets)
            })
            .collect();
            
        GBA {states, delta, initial_state, accepting_transitions}
    }

    fn get_transitions(&self) -> BTreeSet<Self::Transition> {
        self.delta.iter()
            .flat_map(|(source, targets)| {
                targets.iter().map(|(symcon, target)| GBATransition(source.clone(), symcon.clone(), target.clone()))
            })
            .collect()
    }

    fn transition_implies(&self, first: &Self::Transition, second: &Self::Transition) -> bool {
        let GBATransition(qs1, alpha1, q1) = first;
        let GBATransition(qs2, alpha2, q2) = second;

        qs1 == qs2 && alpha2.is_subset(alpha1) && q1 == q2 && self.accepting_transitions.iter().all(|acctranset| {
            acctranset.contains(first) == acctranset.contains(second)
        })
    }

    fn with_only_transitions(self, transitions: BTreeSet<Self::Transition>) -> Self {
        let GBA {states, delta: _, initial_state, accepting_transitions} = self;

        // let states = states_from_transitions(&transitions);
        let delta = agglomerate_transitions(transitions);
        
        GBA {states, delta, initial_state, accepting_transitions}
    }

    fn get_states(&self) -> BTreeSet<Self::State> {
        self.states.clone()
    }

    fn states_equivalent(&self, q1: &Self::State, q2: &Self::State) -> bool {
        self.get_next_edges(q1) == self.get_next_edges(q2)
        && self.get_next_edges(q1).iter().all(|(alpha, qprime)| {
            self.accepting_transitions.iter().all(|acctran| {
                acctran.contains(&GBATransition(q1.clone(), alpha.clone(), qprime.clone())) 
                == acctran.contains(&GBATransition(q2.clone(), alpha.clone(), qprime.clone()))
            })
        })
    }

    fn with_states_merged(self, to_merge: Vec<(Self::State, Self::State)>) -> Self {
        // println!("to_merge length: {}", to_merge.len());
        // println!("{:?}", to_merge);

        let GBA {states, delta, initial_state, accepting_transitions} = self;

        // println!("States pre-merge: {:?}", states);
        // println!("delta length: {}", delta.len());

        let to_remove = to_merge.iter().map(|(_q1, q2)| q2);
        let to_remove_set = to_remove.clone().map(|e| e.clone()).collect::<BTreeSet<_>>();
        let removed_to_new_mapping = to_merge.iter()
            .map(|(fst, snd)| (snd.clone(), fst.clone()))
            .collect::<BTreeMap<_, _>>();

        let initial_state = if removed_to_new_mapping.contains_key(&initial_state) {
            // println!("Replacing initial state");
            removed_to_new_mapping[&initial_state].clone()
        } else {
            // println!("Not replacing initial state");
            initial_state
        };

        let states = to_remove.clone().fold(states, |cstates, cremove| {
            cstates.with_removed(cremove)
        });
        let delta = merge_states_in_transitions(delta, to_remove.clone(), &removed_to_new_mapping);
        let accepting_transitions = accepting_transitions.into_iter()
            .map(|acctranset| {
                acctranset.into_iter()
                    .filter(|GBATransition(source, _action, _sink)| !to_remove_set.contains(source))
                    .map(|GBATransition(source, action, sink)| {
                        let sink = {
                            if let Some(replacement) = removed_to_new_mapping.get(&sink) {
                                replacement.clone()
                            } else {
                                sink
                            }
                        };
                        GBATransition(source, action, sink)
                    })
                    .collect()
            })
            .collect();

        // println!("New delta length: {}", delta.len());
        // println!("States post-merge: {:?}", states);

        GBA {states, delta, initial_state, accepting_transitions}
        
    }
}

fn merge_states_in_transitions<'a, T: Ord + Eq + Clone + 'a + Debug, U: Eq + Ord>(
    transitions: BTreeMap<T, BTreeSet<(U, T)>>,
    to_remove: impl Iterator<Item = &'a T>,
    removed_to_new_mapping: &BTreeMap<T, T>
) -> BTreeMap<T, BTreeSet<(U, T)>> {
    // println!("transitions length: {}", transitions.len());
    to_remove.fold(transitions, |cdelta, cremove| {
        // println!("removing {:?}", cremove);
        cdelta.with_removed(cremove)
    }).into_iter()
        .map(|(source, targets)| {
            let targets = targets.into_iter()
                .map(|(symcon, target)| {
                    if let Some(replacement) = removed_to_new_mapping.get(&target) {
                        (symcon, replacement.clone())
                    } else {
                        (symcon, target)
                    }
                })
                .collect();
            (source, targets)
        })
        .collect::<BTreeMap<_, _>>()
}

impl SimplifiableAutomaton for BA {
    type State = BAState;

    type Transition = (BAState, SymbolConjunction, BAState);

    fn get_initial_states(&self) -> BTreeSet<Self::State> {
        BTreeSet::new().add(self.initial_state.clone())
    }

    fn get_next_states(&self, state: &Self::State) -> BTreeSet<Self::State> {
        // println!("Results:");
        match self.delta.get(&state) {
            Some(results) => results.iter().map(|(_symcon, targets)| {
                // println!("{}", symcon);
                // println!("{:?}", targets);
                targets.clone()
            }).collect(),
            None => BTreeSet::new()
        }
    }

    fn with_only_states(self, states: BTreeSet<Self::State>) -> Self {
        let BA {delta, initial_state, top_layer} = self;

        let delta = delta.into_iter()
            .filter(|(source, _targets)| states.contains(source))
            .map(|(source, targets)| {
                let targets = targets.into_iter()
                    .filter(|(_symcon, target)| states.contains(target))
                    .collect();
                (source, targets)
            })
            .collect();
            
        BA {delta, initial_state, top_layer}
    }

    fn get_transitions(&self) -> BTreeSet<Self::Transition> {
        self.delta.iter()
            .flat_map(|(source, targets)| {
                targets.iter().map(|(symcon, target)| (source.clone(), symcon.clone(), target.clone()))
            })
            .collect()
    }

    fn transition_implies(&self, first: &Self::Transition, second: &Self::Transition) -> bool {
        let (qs1, alpha1, q1) = first;
        let (qs2, alpha2, q2) = second;

        qs1 == qs2 && alpha2.is_subset(&alpha1) && q1 == q2
    }

    fn with_only_transitions(self, transitions: BTreeSet<Self::Transition>) -> Self {
        let BA {delta: _, initial_state, top_layer} = self;

        let delta_prime = transitions.into_iter()
            .fold(BTreeMap::new(), |mut acc, (source, action, sink)| {
                acc.entry(source).or_insert(BTreeSet::new()).insert((action, sink));
                acc
            });
        
        BA {delta: delta_prime, initial_state, top_layer}
    }

    fn get_states(&self) -> BTreeSet<Self::State> {
        self.get_states()
    }

    fn states_equivalent(&self, q1: &Self::State, q2: &Self::State) -> bool {
        self.get_next_edges(q1) == self.get_next_edges(q2)
        && (q1.1 == self.top_layer) == (q2.1 == self.top_layer)
    }

    fn with_states_merged(self, to_merge: Vec<(Self::State, Self::State)>) -> Self {
        // println!("to_merge length: {}", to_merge.len());
        // println!("{:?}", to_merge);

        let BA {delta, initial_state, top_layer: _} = self;

        // println!("delta length: {}", delta.len());

        let to_remove = to_merge.iter().map(|(_q1, q2)| q2);
        let removed_to_new_mapping = to_merge.iter()
            .map(|(fst, snd)| (snd.clone(), fst.clone()))
            .collect::<BTreeMap<_, _>>();

        let initial_state = if removed_to_new_mapping.contains_key(&initial_state) {
            // println!("Replacing initial state");
            removed_to_new_mapping[&initial_state].clone()
        } else {
            // println!("Not replacing initial state");
            initial_state
        };

        let delta = merge_states_in_transitions(delta, to_remove.clone(), &removed_to_new_mapping);
        let top_layer = delta.iter()
            .flat_map(|(source, targets)| {
                Vec::new()
                    .add(source.1)
                    .add_many(targets.iter().map(|(_symcon, BAState(_state, layer))| layer.clone()))
            })
            .max()
            .unwrap_or(0);

        // println!("New delta length: {}", delta.len());

        BA {delta, initial_state, top_layer}
        
    }
}