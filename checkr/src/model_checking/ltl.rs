use std::{collections::HashSet, rc::Rc, fmt::Display};

use crate::ast::BExpr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LTL {
    True,
    Atomic(BExpr),
    And(Box<LTL>, Box<LTL>),
    Not(Box<LTL>),
    Next(Box<LTL>),
    Until(Box<LTL>, Box<LTL>),
    Or(Box<LTL>, Box<LTL>),
    Implies(Box<LTL>, Box<LTL>),
    Iff(Box<LTL>, Box<LTL>),
    Xor(Box<LTL>, Box<LTL>),
    Eventually(Box<LTL>),
    Forever(Box<LTL>)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ReducedLTL {
    True,
    Atomic(BExpr),
    And(Rc<ReducedLTL>, Rc<ReducedLTL>),
    Not(Rc<ReducedLTL>),
    Next(Rc<ReducedLTL>),
    Until(Rc<ReducedLTL>, Rc<ReducedLTL>)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NegativeNormalLTL {
    True,
    False,
    Atomic(BExpr),
    NegAtomic(BExpr),
    And(Rc<NegativeNormalLTL>, Rc<NegativeNormalLTL>),
    Or(Rc<NegativeNormalLTL>, Rc<NegativeNormalLTL>),
    Next(Rc<NegativeNormalLTL>),
    Until(Rc<NegativeNormalLTL>, Rc<NegativeNormalLTL>),
    Release(Rc<NegativeNormalLTL>, Rc<NegativeNormalLTL>),
}

impl LTL {
    pub fn reduced(self) -> ReducedLTL {
        match self {
            LTL::True => ReducedLTL::True,
            LTL::Atomic(bexpr) => ReducedLTL::Atomic(bexpr),
            LTL::And(f1, f2) => ReducedLTL::And(Rc::new(f1.reduced()), Rc::new(f2.reduced())),
            LTL::Not(f) => ReducedLTL::Not(Rc::new(f.reduced())),
            LTL::Next(f) => ReducedLTL::Next(Rc::new(f.reduced())),
            LTL::Until(f1, f2) => ReducedLTL::Until(Rc::new(f1.reduced()), Rc::new(f2.reduced())),
            LTL::Or(f1, f2) =>
                ReducedLTL::Not(
                    Rc::new(ReducedLTL::And(
                        Rc::new(ReducedLTL::Not(Rc::new(f1.reduced()))),
                        Rc::new(ReducedLTL::Not(Rc::new(f2.reduced())))
                    ))
                ),
            LTL::Implies(f1, f2) => {
                let orformula = LTL::Or(
                    Box::new(LTL::Not(Box::new(*f1.clone()))),
                    Box::new(*f2.clone())
                );
                orformula.reduced()
            }
            LTL::Iff(f1, f2) => {
                let implformula = LTL::And(
                    Box::new(LTL::Implies(
                        Box::new(*f1.clone()),
                        Box::new(*f2.clone())
                    )),
                    Box::new(LTL::Implies(
                        Box::new(*f2),
                        Box::new(*f1)
                    ))
                );
                implformula.reduced()
            },
            LTL::Xor(f1, f2) => {
                let orformula = LTL::Or(
                    Box::new(LTL::And(
                        Box::new(*f1.clone()),
                        Box::new(LTL::Not(Box::new(*f2.clone())))
                    )),
                    Box::new(LTL::And(
                        Box::new(*f2),
                        Box::new(LTL::Not(Box::new(*f1)))
                    ))
                );
                orformula.reduced()
            },
            LTL::Eventually(f) => ReducedLTL::Until(Rc::new(ReducedLTL::True), Rc::new(f.reduced())),
            LTL::Forever(f) => {
                let eventuallyformula = LTL::Not(
                    Box::new(LTL::Eventually(
                        Box::new(LTL::Not(
                            Box::new(*f)
                        ))
                    ))
                );
                eventuallyformula.reduced()
            }
        }
    }
}

impl ReducedLTL {
    pub fn unreduced(&self) -> LTL {
        match self {
            ReducedLTL::True => LTL::True,
            ReducedLTL::Atomic(bexpr) => LTL::Atomic(bexpr.clone()),
            ReducedLTL::And(f1, f2) => LTL::And(Box::new(f1.unreduced()), Box::new(f2.unreduced())),
            ReducedLTL::Not(f) => LTL::Not(Box::new(f.unreduced())),
            ReducedLTL::Next(f) => LTL::Next(Box::new(f.unreduced())),
            ReducedLTL::Until(f1, f2) => LTL::Until(Box::new(f1.unreduced()), Box::new(f2.unreduced())),
        }
    }

    pub fn to_negative_normal(&self) -> NegativeNormalLTL {
        match self {
            ReducedLTL::True => NegativeNormalLTL::True,
            ReducedLTL::Atomic(bexpr) => NegativeNormalLTL::Atomic(bexpr.clone()),
            ReducedLTL::And(f1, f2) => NegativeNormalLTL::And(Rc::new(f1.to_negative_normal()), Rc::new(f2.to_negative_normal())),
            ReducedLTL::Next(f) => NegativeNormalLTL::Next(Rc::new(f.to_negative_normal())),
            ReducedLTL::Until(f1, f2) => NegativeNormalLTL::Until(Rc::new(f1.to_negative_normal()), Rc::new(f2.to_negative_normal())),
            ReducedLTL::Not(f) =>
                match &**f {
                    ReducedLTL::True => NegativeNormalLTL::False,
                    ReducedLTL::Not(f1) => f1.to_negative_normal(),
                    ReducedLTL::And(f1, f2) => NegativeNormalLTL::Or(
                        Rc::new(ReducedLTL::Not(Rc::clone(f1)).to_negative_normal()),
                        Rc::new(ReducedLTL::Not(Rc::clone(f2)).to_negative_normal())
                    ),
                    ReducedLTL::Next(f1) => NegativeNormalLTL::Next(
                        Rc::new(ReducedLTL::Not(Rc::clone(f1)).to_negative_normal())
                    ),
                    ReducedLTL::Until(f1, f2) => NegativeNormalLTL::Release(
                        Rc::new(ReducedLTL::Not(Rc::clone(f1)).to_negative_normal()),
                        Rc::new(ReducedLTL::Not(Rc::clone(f2)).to_negative_normal())
                    ),
                    ReducedLTL::Atomic(f1) => NegativeNormalLTL::NegAtomic(f1.clone())
                }

        }
    }
}

pub fn temporal_subformulae(formula: &NegativeNormalLTL) -> HashSet<NegativeNormalLTL> {
    let mut set = HashSet::new();
    match formula {
        NegativeNormalLTL::True | NegativeNormalLTL::False | NegativeNormalLTL::Atomic(_) | NegativeNormalLTL::NegAtomic(_) => {
            set.insert(formula.clone());
        },
        NegativeNormalLTL::And(f1, f2) => {
            set.extend(temporal_subformulae(f1));
            set.extend(temporal_subformulae(f2));
        }
        NegativeNormalLTL::Or(f1, f2) => {
            set.extend(temporal_subformulae(f1));
            set.extend(temporal_subformulae(f2));
        }
        NegativeNormalLTL::Next(f) => {
            set.insert(formula.clone());
            set.extend(temporal_subformulae(f));
        }
        NegativeNormalLTL::Until(f1, f2) => {
            set.insert(formula.clone());
            set.extend(temporal_subformulae(f1));
            set.extend(temporal_subformulae(f2));
        }
        NegativeNormalLTL::Release(f1, f2) => {
            set.insert(formula.clone());
            set.extend(temporal_subformulae(f1));
            set.extend(temporal_subformulae(f2));
        }
    }
    set
}

pub fn until_subformulae(formula: &NegativeNormalLTL) -> HashSet<NegativeNormalLTL> {
    let mut set = HashSet::new();
    match formula {
        NegativeNormalLTL::True | NegativeNormalLTL::False | NegativeNormalLTL::Atomic(_) | NegativeNormalLTL::NegAtomic(_) => {},
        NegativeNormalLTL::And(f1, f2) => {
            set.extend(until_subformulae(f1));
            set.extend(until_subformulae(f2));
        }
        NegativeNormalLTL::Or(f1, f2) => {
            set.extend(until_subformulae(f1));
            set.extend(until_subformulae(f2));
        }
        NegativeNormalLTL::Next(f) => {
            set.extend(until_subformulae(f));
        }
        NegativeNormalLTL::Until(f1, f2) => {
            set.insert(formula.clone());
            set.extend(until_subformulae(f1));
            set.extend(until_subformulae(f2));
        }
        NegativeNormalLTL::Release(f1, f2) => {
            set.extend(until_subformulae(f1));
            set.extend(until_subformulae(f2));
        }
    }
    set
}

// fn half_closure(formula: &Rc<ReducedLTL>) -> HashSet<Rc<ReducedLTL>> {
//     let mut subformulas = HashSet::new();
//     subformulas.insert(Rc::clone(formula));
//     // subformulas.insert(Rc::new(ReducedLTL::Not(Rc::clone(formula))));

//     match &**formula {
//         ReducedLTL::True | ReducedLTL::Atomic(_) => {},
//         ReducedLTL::And(f1, f2) => {
//             subformulas.extend(half_closure(&f1));
//             subformulas.extend(half_closure(&f2));
//         },
//         ReducedLTL::Not(f) => subformulas.extend(half_closure(&f)),
//         ReducedLTL::Next(f) => subformulas.extend(half_closure(&f)),
//         ReducedLTL::Until(f1, f2) => {
//             subformulas.extend(half_closure(&f1));
//             subformulas.extend(half_closure(&f2));
//         }
//     }

//     subformulas
// }

impl Display for NegativeNormalLTL {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NegativeNormalLTL::True => write!(f, "tt"),
            NegativeNormalLTL::False => write!(f, "ff"),
            NegativeNormalLTL::Atomic(b) => write!(f, "{b}"),
            NegativeNormalLTL::NegAtomic(b) => write!(f, "!{b}"),
            NegativeNormalLTL::And(f1, f2) => write!(f, "({f1} && {f2})"),
            NegativeNormalLTL::Or(f1, f2) => write!(f, "({f1} || {f2})"),
            NegativeNormalLTL::Next(f1) => write!(f, "O{f1}"),
            NegativeNormalLTL::Until(f1, f2) => write!(f, "({f1} U {f2})"),
            NegativeNormalLTL::Release(f1, f2) => write!(f, "({f1} R {f2})"),
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eventually_reduced() {
        let reduced = LTL::Eventually(Box::new(LTL::Atomic(BExpr::Bool(false)))).reduced();
        assert_eq!(reduced, ReducedLTL::Until(Rc::new(ReducedLTL::True), Rc::new(ReducedLTL::Atomic(BExpr::Bool(false)))));
    }
}