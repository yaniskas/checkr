use std::collections::HashMap;

use itertools::Itertools;

use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::{
    ast::{Commands, Variable},
    generation::Generate,
    security::{Flow, SecurityAnalysisResult, SecurityClass, SecurityLattice},
};

use super::{Environment, ToMarkdown, ValidationResult};

#[derive(Debug)]
pub struct SecurityEnv;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityLatticeInput(Vec<Flow<SecurityClass>>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityAnalysisInput {
    pub classification: HashMap<Variable, SecurityClass>,
    pub lattice: SecurityLatticeInput,
}

impl Generate for SecurityAnalysisInput {
    type Context = Commands;

    fn gen<R: rand::Rng>(cx: &mut Self::Context, rng: &mut R) -> Self {
        let classification = cx
            .fv()
            .into_iter()
            .map(|v| {
                (
                    v,
                    [
                        SecurityClass("A".to_string()),
                        SecurityClass("B".to_string()),
                        SecurityClass("C".to_string()),
                        SecurityClass("D".to_string()),
                    ]
                    .choose(rng)
                    .unwrap()
                    .clone(),
                )
            })
            .collect();
        let lattice = SecurityLatticeInput(vec![
            Flow {
                from: SecurityClass("A".to_string()),
                into: SecurityClass("B".to_string()),
            },
            Flow {
                from: SecurityClass("C".to_string()),
                into: SecurityClass("D".to_string()),
            },
        ]);

        SecurityAnalysisInput {
            classification,
            lattice,
        }
    }
}

impl ToMarkdown for SecurityAnalysisInput {
    fn to_markdown(&self) -> String {
        format!(
            "Lattice: {}\n\nClassification: [{}]",
            self.lattice
                .0
                .iter()
                .map(|f| format!("{} < {}", f.from, f.into))
                .format(", "),
            self.classification
                .iter()
                .map(|(a, c)| format!("{a} = {c}"))
                .format(", ")
        )
    }
}

impl Environment for SecurityEnv {
    type Input = SecurityAnalysisInput;

    type Output = SecurityAnalysisResult;

    fn command() -> &'static str {
        "security"
    }
    fn name(&self) -> String {
        "Security Analysis".to_string()
    }

    fn run(&self, cmds: &Commands, input: &Self::Input) -> Self::Output {
        let lattice = SecurityLattice::new(&input.lattice.0);
        SecurityAnalysisResult::run(&input.classification, &lattice, cmds)
    }

    fn validate(
        &self,
        cmds: &Commands,
        input: &Self::Input,
        output: &Self::Output,
    ) -> ValidationResult
    where
        Self::Output: PartialEq + std::fmt::Debug,
    {
        let mut reference = self.run(cmds, input);
        reference.actual.sort();
        reference.allowed.sort();
        reference.violations.sort();
        let mut output = output.clone();
        output.actual.sort();
        output.allowed.sort();
        output.violations.sort();

        if reference == output {
            ValidationResult::CorrectTerminated
        } else {
            ValidationResult::Mismatch {
                reason: format!("{input:?}\n{cmds}\n{reference:#?} != {output:#?}"),
            }
        }
    }
}