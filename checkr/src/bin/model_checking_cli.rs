use std::fs;

use checkr::{parse, pg::Determinism, model_checking::stuck_states::check_model, concurrency::ParallelProgramGraph, util::cli_utils::{ask_for_with_parser, ask_for, ask_for_memory_assignment}};

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

    let pg_dot = graph.dot();
    fs::write("graphviz_output/program_graph.dot", pg_dot).unwrap();
    println!("Wrote program graph to graphviz_output/program_graph.dot");

    let memory = ask_for_memory_assignment(commands);

    let search_depth = ask_for::<u64>(
        "Please enter the desired search depth: ",
        "Please enter a valid positive number"
    );

    let result = check_model(search_depth, memory, &graph);
    let stuck_states = result.stuck_states;
    let transition_system = if graph.num_processes() == 1 {
        result.transition_system.into_iter()
            .map(|(config, outgoing)| {
                (
                    config.make_node_non_parallel(),
                    outgoing.into_iter().map(|(action, target)| (action, target.make_node_non_parallel())).collect()
                )
            })
            .collect()
    } else {result.transition_system};
    
    if !stuck_states.is_empty() {
        println!("Stuck states:");
        for pt in stuck_states {
            println!("{pt:#?}");
        }
    } else {
        println!("No stuck states found");
    }

    let mut graphviz_edges = transition_system.iter()
        .flat_map(|entry| {
            entry.1.iter().map(move |edge| {
                format!("\"{}\" -> \"{}\" [label = \"{}\"]", entry.0, edge.1, edge.0)
            })
        }).collect::<Vec<_>>();
    graphviz_edges.sort();
    let graphviz_edges_str = graphviz_edges.join("\n").replace("▷", "Start").replace("◀", "End");

    println!("Transition system edges:");
    println!("{}", graphviz_edges_str);

    let graphviz_output =
        "digraph transition_system {\n".to_string()
        + &graphviz_edges_str
        + "}";
    fs::write("graphviz_output/transition_system.dot", graphviz_output).unwrap();
    println!("Wrote transition system to graphviz_output/transition_system.dot")
}