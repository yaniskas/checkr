use std::{io::{self, Write}, collections::BTreeMap, str::FromStr, fs};

use checkr::{parse, pg::{ProgramGraph, Determinism}, ast::{Variable, Target, Array, Commands, BExpr, AExpr, RelOp}, model_checking::{ModelCheckMemory, check_model, ltl_ast::LTL, vwaa::{VWAA, LTLConjunction}, gba::{GBA, GBATransition}, simplification::SimplifiableAutomaton, ba::BA, nested_dfs::{nested_dfs, LTLVerificationResult}}};
use itertools::Itertools;

fn main() {
    let program = "
    i := 0;
    do i < 5 ->
        if true ->
            n := n + 1
        [] true ->
            n := n - 1
        fi
    od
    ";

    // let program = "
    // n := 0;
    // n := -1;
    // ";

    // let program = "
    // n := -1;
    // do true -> n := n + 1 od
    // ";

    // let program = "
    // n := 3;
    // ";

    let commands = parse::parse_commands(program).unwrap();
    let graph = ProgramGraph::new(Determinism::NonDeterministic, &commands);

    let memory = ModelCheckMemory {
        variables: vec![(Variable("i".to_string()), 0), (Variable("n".to_string()), 0)].into_iter().collect(),
        arrays: BTreeMap::new(),
    };

    let initial_formula = LTL::Forever(Box::new(
        LTL::Atomic(BExpr::Rel(AExpr::Reference(Target::Variable(Variable("n".to_string()))), RelOp::Ge, AExpr::Number(0)))
    ));
    // let initial_formula = LTL::Forever(Box::new(
    //     LTL::Eventually(Box::new(
    //         LTL::Atomic(BExpr::Rel(AExpr::Reference(Target::Variable(Variable("n".to_string()))), RelOp::Eq, AExpr::Number(3)))
    //     ))
    // ));
    let search_depth = 60;

    // Negate the formula before converting into an automaton
    let formula = LTL::Not(Box::new(initial_formula));

    let reduced = dbg!(formula.reduced());
    let nn = dbg!(reduced.to_negative_normal());
    println!("Negative normal: {:?}", nn);
    println!("\n\n\n\n\nCreating VWAA");
    let vwaa = dbg!(VWAA::from_ltl(&nn));

    let vwaa_edges = vwaa.delta.iter()
        .flat_map(|(source, targets)| {
            targets.iter().flat_map(move |(symcon, ltlcon)| {
                if ltlcon.is_true() {
                    vec! [format!("\"{}\" -> \"{}\" [label = \"{}\"]", source, "tt", symcon)]
                } else {
                    ltlcon.get_raw_components().iter().map(move |ltl| {
                        format!("\"{}\" -> \"{}\" [label = \"{}\"]", source, ltl, symcon)
                    }).collect()
                }
            })
        }).collect::<Vec<_>>();
    let vwaa_edges_str = vwaa_edges.join("\n");

    println!("VWAA edges:");
    println!("{}", vwaa_edges_str);

    let graphviz_output =
        "digraph vwaa {\n".to_string()
        + &vwaa_edges_str
        + "}";
    fs::write("graphviz_output/ltl_test/vwaa.dot", graphviz_output).unwrap();


    // GBA
    println!("\n\n\n\n\nCreating GBA");
    let gba = GBA::from_vwaa(vwaa);

    println!("{}", gba.states.iter().map(|e| e).join(",\n"));

    let gba_edges = gba.delta.iter()
        .flat_map(|(source, targets)| {
            targets.iter().map(move |(symcon, target)| {
                format!("\"{}\" -> \"{}\" [label = \"{}\"]", source, target, symcon)
            })
        }).collect::<Vec<_>>();
    let gba_edges_str = gba_edges.join("\n");

    println!("GBA edges:");
    println!("{}", gba_edges_str);

    let gba_output =
        "digraph gba {\n".to_string()
        + &gba_edges_str
        + "}";
    fs::write("graphviz_output/ltl_test/gba.dot", gba_output).unwrap();


    // Simplified GBA
    println!("\n\n\n\n\nCreating simplified GBA");
    let simplified_gba = gba.simplify();

    println!("{}", simplified_gba.states.iter().map(|e| e).join(",\n"));

    let simplified_gba_edges = simplified_gba.delta.iter()
        .flat_map(|(source, targets)| {
            targets.iter().map(move |(symcon, target)| {
                format!("\"{}\" -> \"{}\" [label = \"{}\"]", source, target, symcon)
            })
        }).collect::<Vec<_>>();
    let simplified_gba_edges_str = simplified_gba_edges.join("\n");

    println!("Simplified GBA edges:");
    println!("{}", simplified_gba_edges_str);

    //
    println!("Simplified GBA accepting transitions:");
    for (i, acc_tran_set) in simplified_gba.accepting_transitions.iter().enumerate() {
        println!("Set {i}");
        for GBATransition(source, action, target) in acc_tran_set {
            println!("source: {}, action: {}, target: {}", source, action, target);
        }
    }

    let simplified_gba_output =
        "digraph simplified_gba {\n".to_string()
        + &simplified_gba_edges_str
        + "}";
    fs::write("graphviz_output/ltl_test/gba_simplified.dot", simplified_gba_output).unwrap();


    // BA
    println!("\n\n\n\n\nCreating BA");
    let ba = BA::from_gba(simplified_gba);
    
    let ba_edges = ba.delta.iter()
        .flat_map(|(source, targets)| {
            targets.iter().map(move |(symcon, target)| {
                format!("\"{}\" -> \"{}\" [label = \"{}\"]", source, target, symcon)
            })
        }).collect::<Vec<_>>();
    let ba_edges_str = ba_edges.join("\n");

    println!("BA edges:");
    println!("{}", ba_edges_str);

    let initial_state_str = format!("node [shape = doublecircle]; \"{}\"\nnode [shape = circle]\n", ba.initial_state);

    let ba_output =
        "digraph ba {\n".to_string()
        + &initial_state_str
        + &ba_edges_str
        + "}";
    fs::write("graphviz_output/ltl_test/ba.dot", ba_output).unwrap();



    // Simplified BA
    println!("\n\n\n\n\nCreating simplified BA");
    let simplified_ba = ba.simplify();
    
    let simplified_ba_edges = simplified_ba.delta.iter()
        .flat_map(|(source, targets)| {
            targets.iter().map(move |(symcon, target)| {
                format!("\"{}\" -> \"{}\" [label = \"{}\"]", source, target, symcon)
            })
        }).collect::<Vec<_>>();
    let simplified_ba_edges_str = simplified_ba_edges.join("\n");

    println!("Simplified BA edges:");
    println!("{}", simplified_ba_edges_str);

    let initial_state_str = format!("node [shape = doublecircle]; \"{}\"\nnode [shape = circle]\n", simplified_ba.initial_state);

    let simplified_ba_output =
        "digraph ba_simplified {\n".to_string()
        + &initial_state_str
        + &simplified_ba_edges_str
        + "}";
    fs::write("graphviz_output/ltl_test/ba_simplified.dot", simplified_ba_output).unwrap();

    // Model checking
    println!("\n\n\n\n\nChecking LTL formula");

    let trace = nested_dfs(&graph, &simplified_ba, &memory, search_depth);
    match trace {
        LTLVerificationResult::CycleFound(trace) => {
            println!("Violating trace found:");
            for (config, bastate) in trace {
                println!("{:?}, {:?}", config, bastate);
            }
        }
        LTLVerificationResult::CycleNotFound => {
            println!("No violating trace found");
        }
        LTLVerificationResult::SearchDepthExceeded => {
            println!("Search depth exceeded");
        }
    }
}