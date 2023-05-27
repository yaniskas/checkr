use checkr::{parse, pg::Determinism, model_checking::{ltl_ast::parse_ltl, ltl_verification::{verify_ltl, zero_initialized_memory}, nested_dfs::LTLVerificationResult}, concurrency::ParallelProgramGraph, util::cli_utils::{ask_for_with_parser, ask_for}};

fn main() {
    let commands = ask_for_with_parser(
        "Please enter a GCL program: ", 
        "Please enter a valid program", 
        parse::parse_parallel_commands
    );

    let det_choice = ask_for::<usize>(
        "Please enter 0 for the program to be non-deterministic, or 1 for it to be deterministic: ",
        "Please enter a valid number"
    );

    let det = match det_choice {
        0 => Determinism::NonDeterministic,
        _ => Determinism::Deterministic,
    };

    let graph = ParallelProgramGraph::new(det, &commands);

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
        LTLVerificationResult::CycleFound{trace, cycle_start} => {
            println!("Formula not satisfied");
            println!("Violating trace:");
            for (i, (_action, (config, _bastate))) in trace.iter().enumerate() {
                println!("{}{}", config, if i == cycle_start {" <------ START OF CYCLE"} else {""});
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