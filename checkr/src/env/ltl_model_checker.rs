use itertools::{chain, Itertools};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::{
    ast::{Commands, Command, ParallelCommands, ModelCheckingArgs, FullAssignment},
    generation::Generate,
    interpreter::{Configuration, Interpreter, InterpreterMemory, TerminationState},
    pg::{Determinism, Node, ProgramGraph},
    sign::{Memory, MemoryRef},
    ValidationResult::CorrectTerminated,
    concurrency::{ParallelProgramGraph, ParallelConfiguration},
    model_checking::{ltl_verification::{verify_ltl, zero_initialized_memory}, nested_dfs::LTLVerificationResult},
};

use super::{Analysis, EnvError, Environment, Markdown, ToMarkdown, ValidationResult};

#[derive(Debug)]
pub struct ModelCheckerEnv;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCheckerInput;

impl Generate for ModelCheckerInput {
    type Context = Commands;

    fn gen<R: rand::Rng>(cx: &mut Self::Context, mut rng: &mut R) -> Self {
        ModelCheckerInput
    }
}

impl ToMarkdown for ModelCheckerInput {
    fn to_markdown(&self) -> Markdown {
        Markdown(String::new())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelCheckerOutput {
    FormulaHolds,
    FormulaDoesNotHold(Vec<ParallelConfiguration>),
    SearchDepthExceeded,
    FormulaMissing,
    InvalidSearchDepth,
}

impl ToMarkdown for ModelCheckerOutput {
    fn to_markdown(&self) -> Markdown {
        match self {
            ModelCheckerOutput::FormulaHolds => Markdown("The formula holds".to_string()),
            ModelCheckerOutput::SearchDepthExceeded => Markdown("Search depth exceeded".to_string()),
            ModelCheckerOutput::FormulaDoesNotHold(configs) => {
                let variables = configs
                    .iter()
                    .flat_map(|t| t.memory.variables.keys().map(|k| k.to_string()))
                    .sorted()
                    .dedup()
                    .collect_vec();
                let arrays = configs
                    .iter()
                    .flat_map(|t| t.memory.arrays.keys().map(|k| k.to_string()))
                    .sorted()
                    .dedup()
                    .collect_vec();
        
                let mut table = comfy_table::Table::new();
                table
                    .load_preset(comfy_table::presets::ASCII_MARKDOWN)
                    .set_header(chain!(
                        (0..configs[0].nodes.len()).into_iter().map(|num| format!("Process {}", num)),
                        variables.iter().cloned(),
                        arrays.iter().cloned()
                    ));

                for t in configs {
                    table.add_row(chain!(
                        t.nodes.iter().map(ToString::to_string),
                        chain!(
                            t.memory
                                .variables
                                .iter()
                                .map(|(var, value)| (value.to_string(), var.to_string()))
                                .sorted_by_key(|(_, k)| k.to_string()),
                            t.memory
                                .arrays
                                .iter()
                                .map(|(arr, values)| {
                                    (format!("[{}]", values.iter().format(",")), arr.to_string())
                                })
                                .sorted_by_key(|(_, k)| k.to_string()),
                        )
                        .map(|(v, _)| v),
                    ));
                }
                // let final_message = match self.final_state {
                //     TerminationState::Running => {
                //         format!("**Stopped after {} steps**", self.execution_sequence.len())
                //     }
                //     TerminationState::Stuck => "**Stuck**".to_string(),
                //     TerminationState::Terminated => "**Terminated successfully**".to_string(),
                // };
                // table.add_row([final_message]);
        
                format!("The formula does not hold\n\nViolating trace:\n{table}").into()
            }
            ModelCheckerOutput::FormulaMissing => Markdown("Please type \"ltl\" followed by an LTL formula after the program".to_string()),
            ModelCheckerOutput::InvalidSearchDepth => Markdown("Please input a search depth greater than 0".to_string()),
        }

    }
}

impl Environment for ModelCheckerEnv {
    type Input = ModelCheckerInput;

    type Output = ModelCheckerOutput;

    const ANALYSIS: Analysis = Analysis::LTLModelChecking;

    fn run(&self, cmds: &Commands, input: &Self::Input) -> Result<Self::Output, EnvError> {
        if cmds.0.len() == 0 {panic!("Not enough information to parse")}

        let args = if let Command::ModelCheckingArgs(args) = &cmds.0[0] {
            args.clone()
        } else {
            return Ok(ModelCheckerOutput::FormulaMissing)
        };

        let parallel_commands = match &cmds.0[1] {
            Command::Parallel(res) => res.clone(),
            _ => ParallelCommands(vec![Commands((&cmds.0[1..]).iter().map(Clone::clone).collect())]),
        };
        
        let ModelCheckingArgs{initial_assignment, determinism, ltl, search_depth} = args;

        let determinism = if determinism == Some(true) {Determinism::Deterministic} else {Determinism::NonDeterministic};
        
        let graph = ParallelProgramGraph::new(determinism, &parallel_commands);
        
        let mut memory = zero_initialized_memory(&graph, 10);

        if let Some(initial_assignment) = initial_assignment {
            for assignment in initial_assignment {
                match assignment {
                    FullAssignment::VariableAssignment(name, value) => {
                        memory.variables.insert(name, value);
                    }
                    FullAssignment::ArrayAssignment(name, value) => {
                        memory.arrays.insert(name, value);
                    }
                }
            }
        }

        let search_depth = match search_depth {
            Some(val) if val > 0 => val as usize,
            None => 100,
            _ => return Ok(ModelCheckerOutput::InvalidSearchDepth),
        };

        let res = verify_ltl(&graph, ltl, &memory, search_depth);

        match res {
            LTLVerificationResult::CycleFound(trace) => {
                Ok(ModelCheckerOutput::FormulaDoesNotHold(trace.into_iter()
                    .map(|(_action, (config, _ba_state))| config).collect()
                ))
            }
            LTLVerificationResult::CycleNotFound => Ok(ModelCheckerOutput::FormulaHolds),
            LTLVerificationResult::SearchDepthExceeded => Ok(ModelCheckerOutput::SearchDepthExceeded),
        }
    }

    fn validate(
        &self,
        _: &Commands,
        _: &Self::Input,
        _: &Self::Output,
    ) -> Result<ValidationResult, EnvError>
    where
        Self::Output: PartialEq,
    { Ok(CorrectTerminated) }
}