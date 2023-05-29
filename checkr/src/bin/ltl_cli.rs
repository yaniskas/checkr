use checkr::{parse, pg::Determinism, model_checking::{ltl_ast::parse_ltl, ltl_verification::{verify_ltl, zero_initialized_memory}, nested_dfs::LTLVerificationResult}, concurrency::ParallelProgramGraph, util::cli_utils::{ask_for_with_parser, ask_for, parse_or_nothing, parse_memory_assignment, parse_bool, parse_positive_nonzero_int}, ast::FullAssignment};

fn main() {
    let commands = ask_for_with_parser(
        "Please enter a GCL program: ", 
        "Please enter a valid program", 
        parse::parse_parallel_commands
    );

    let initial_assignment = ask_for_with_parser(
        "Please enter the desired initial memory, or press ENTER to use a zero-initialized memory: ",
        "Please enter a valid memory assignment",
        parse_or_nothing(parse_memory_assignment)
    );

    let det_choice = ask_for_with_parser(
        "Please enter true for the program to be deterministic, or, either enter false or press ENTER for it to be non-deterministic: ",
        "Please enter a valid option",
        parse_or_nothing(parse_bool)
    );

    let det = if det_choice == Some(true) {Determinism::Deterministic} else {Determinism::NonDeterministic};

    let graph = ParallelProgramGraph::new(det, &commands);

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
    
    let ltl = ask_for_with_parser(
        "Please enter the desired LTL formula: ", 
        "Please enter a valid formula", 
        parse_ltl
    );

    let search_depth = ask_for_with_parser(
        "Please enter the desired search depth, or press ENTER to use the default value of 100: ",
        "Please enter a valid number greater than 0",
        parse_or_nothing(parse_positive_nonzero_int)
    );

    let search_depth = match search_depth {
        Some(val) => val as usize,
        None => 100,
    };

    let res = verify_ltl(&graph, ltl, &memory, search_depth);
    match res {
        LTLVerificationResult::CycleFound{trace, cycle_start} => {
            println!("Formula not satisfied, violating cycle found");
            println!("Violating trace:");
            for (i, (_action, (config, _bastate))) in trace.iter().enumerate() {
                println!("{}{}", config, if i == cycle_start {" <------ START OF CYCLE"} else {""});
            }
        }
        LTLVerificationResult::ViolatingStateReached{trace} => {
            println!("Formula not satisfied, violating state found");
            println!("Violating trace:");
            for (_action, (config, _bastate)) in trace {
                println!("{}", config);
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