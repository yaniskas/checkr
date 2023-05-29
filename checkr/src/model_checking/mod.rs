use crate::interpreter::InterpreterMemory;

pub mod stuck_states;
pub mod ltl_ast;
pub mod vwaa;
pub mod gba;
pub mod simplification;
pub mod nba;
pub mod nested_dfs;
pub mod ltl_verification;

pub type ModelCheckMemory = InterpreterMemory;