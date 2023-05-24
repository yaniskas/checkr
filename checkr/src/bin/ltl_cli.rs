use std::{io::{self, Write}, collections::BTreeMap, str::FromStr, fs};

use checkr::{parse, pg::{ProgramGraph, Determinism}, ast::{Variable, Target, Array, Commands}, model_checking::{ModelCheckMemory, check_model, ltl_ast::parse_ltl, ltl_verification::{verify_ltl, zero_initialized_memory}, nested_dfs::LTLVerificationResult}, concurrency::ParallelProgramGraph};

fn main() {
    let commands = ask_for_with_parser(
        "Please enter a GCL program: ", 
        "Please enter a valid program", 
        parse::parse_parallel_commands
    );
    let graph = ParallelProgramGraph::new(Determinism::NonDeterministic, &commands);

    let array_length = ask_for::<usize>(
        "Please enter the desired length of arrays: ",
        "Please enter a valid positive number"
    );
    let memory = zero_initialized_memory(&graph, array_length);

    let search_depth = ask_for::<usize>(
        "Please enter the desired search depth: ",
        "Please enter a valid positive number"
    );

    let ltl = ask_for_with_parser(
        "Please enter the desired LTL formula: ", 
        "Please enter a valid formula", 
        parse_ltl
    );

    let res = verify_ltl(&graph, ltl, &memory, search_depth);
    match res {
        LTLVerificationResult::CycleFound(cyc) => {
            println!("Formula not satisfied");
            println!("Violating trace:");
            for (action, config) in cyc {
                println!("{} {:?}", action, config);
            }
        }
        LTLVerificationResult::SearchDepthExceeded => {
            println!("Search depth too low, could not verify formula");
        }
        LTLVerificationResult::CycleNotFound => {
            println!("No violating trace found, the formula holds");
        }
    }
}

fn ask_for<T: FromStr>(msg: &str, failmsg: &str) -> T {
    ask_for_with_parser(msg, failmsg, FromStr::from_str)
}

fn ask_for_with_parser<T, E>(msg: &str, failmsg: &str, parser: impl Fn(&str) -> Result<T, E>) -> T {
    loop {
        match parser(input_msg(msg).trim()) {
            Ok(val) => return val,
            Err(_) => {
                println!("{failmsg}");
                continue;
            }
        }
    }
}

fn ask_for_memory_assignment(commands: Commands) -> ModelCheckMemory {
    let assignment_targets = commands.fv();
    let mut var_map = BTreeMap::new();
    let mut arr_map = BTreeMap::new();

    for t in assignment_targets {
        match t {
            Target::Variable(wrapped_name) => {
                let Variable(name) = &wrapped_name;

                let desired_value = ask_for::<i64>(
                    &format!("Please input an initial assignment for variable {name}: "),
                    "Please enter a valid number"
                );

                var_map.insert(wrapped_name, desired_value);
            }
            Target::Array(wrapped_name, _) => {
                let Array(name) = &wrapped_name;

                let desired_value: Vec<i64> = loop {
                    let input = input_msg(&format!("Please input an initial assignment for array {name}, with values separated by commas: "));
                    match try_parse_array(&input) {
                        Ok(ass) => break ass,
                        Err(_) => {
                            println!("Please enter a valid array");
                            continue;
                        }
                    }
                };
                
                arr_map.insert(wrapped_name, desired_value);
            }
        }
    }

    ModelCheckMemory {
        variables: var_map,
        arrays: arr_map
    }
}

fn input() -> String {
    let mut str = String::new();
    io::stdin().read_line(&mut str).expect("Failed to read line");
    str
}

fn input_msg(message: &str) -> String {
    print!("{message}");
    io::stdout().flush().expect("Error flushing");
    input()
}

fn try_parse_array(string: &str) -> Result<Vec<i64>, ()> {
    let nums: Vec<Result<i64, _>> = string.split(",").map(|e| e.trim().parse()).collect();
    if nums.iter().all(|r| r.is_ok()) { Ok(nums.into_iter().map(|r| r.unwrap()).collect()) }
    else { Err(()) }
}