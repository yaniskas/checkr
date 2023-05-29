use std::{fs};

use checkr::{model_checking::{ltl_ast::{LTL, parse_ltl}, vwaa::VWAA, gba::{GBA, GBATransition}, simplification::SimplifiableAutomaton, nba::NBA}, util::cli_utils::{ask_for_with_parser, initial_state_arrow, initial_state_arrow_num}};
use itertools::Itertools;


const VWAA_PATH: &str = "graphviz_output/vwaa.dot";
const GBA_PATH: &str = "graphviz_output/gba.dot";
const SIMPLIFIED_GBA_PATH: &str = "graphviz_output/gba_simplified.dot";
const NBA_PATH: &str = "graphviz_output/nba.dot";
const SIMPLIFIED_NBA_PATH: &str = "graphviz_output/nba_simplified.dot";

fn main() {
    let input_ltl = ask_for_with_parser(
        "Please enter the desired LTL formula: ", 
        "Please enter a valid formula", 
        parse_ltl
    );

    // let formula = dbg!(LTL::Not(Box::new(input_ltl)));
    let formula = LTL::Not(Box::new(input_ltl));

    // let reduced = dbg!(formula.reduced());
    let reduced = formula.reduced();
    // let nn = dbg!(reduced.to_negative_normal());
    let nn = reduced.to_negative_normal();
    println!("\n\n\n\n\nCreating VWAA");
    // let vwaa = dbg!(VWAA::from_ltl(&nn));
    let vwaa = VWAA::from_ltl(&nn);

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
        + &vwaa.initial_states.iter().enumerate().map(|(i, state)| initial_state_arrow_num(state, i)).join("\n")
        + &vwaa_edges_str
        + "}";
    fs::create_dir_all("graphviz_output").unwrap();
    fs::write(VWAA_PATH, graphviz_output).unwrap();
    println!("Wrote VWAA to {VWAA_PATH}");


    // GBA
    println!("\n\n\n\n\nCreating GBA");
    let gba = GBA::from_vwaa(vwaa);

    println!("GBA states: {}", gba.states.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(",\n"));

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
        + &initial_state_arrow(&gba.initial_state)
        + &gba_edges_str
        + "}";
    fs::write(GBA_PATH, gba_output).unwrap();
    println!("Wrote GBA to {GBA_PATH}");


    // Simplified GBA
    println!("\n\n\n\n\nCreating simplified GBA");
    let simplified_gba = gba.simplify();

    println!("Simplified GBA states: {}", simplified_gba.states.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(",\n"));

    let simplified_gba_edges = simplified_gba.delta.iter()
        .flat_map(|(source, targets)| {
            targets.iter().map(move |(symcon, target)| {
                format!("\"{}\" -> \"{}\" [label = \"{}\"]", source, target, symcon)
            })
        }).collect::<Vec<_>>();
    let simplified_gba_edges_str = simplified_gba_edges.join("\n");

    println!("Simplified GBA edges:");
    println!("{}", simplified_gba_edges_str);

    println!("Simplified GBA accepting transitions:");
    for (i, acc_tran_set) in simplified_gba.accepting_transitions.iter().enumerate() {
        println!("Set {i}");
        for GBATransition(source, action, target) in acc_tran_set {
            println!("source: {}, action: {}, target: {}", source, action, target);
        }
    }

    let simplified_gba_output =
        "digraph simplified_gba {\n".to_string()
        + &initial_state_arrow(&simplified_gba.initial_state)
        + &simplified_gba_edges_str
        + "}";
    fs::write(SIMPLIFIED_GBA_PATH, simplified_gba_output).unwrap();
    println!("Wrote simplified GBA to {SIMPLIFIED_GBA_PATH}");



    // NBA
    println!("\n\n\n\n\nCreating NBA");
    let nba = NBA::from_gba(simplified_gba);
    
    let nba_edges = nba.delta.iter()
        .flat_map(|(source, targets)| {
            targets.iter().map(move |(symcon, target)| {
                format!("\"{}\" -> \"{}\" [label = \"{}\"]", source, target, symcon)
            })
        }).collect::<Vec<_>>();
    let nba_edges_str = nba_edges.join("\n");

    println!("NBA initial state: {}", nba.initial_state);
    println!("NBA edges:");
    println!("{}", nba_edges_str);

    let nba_output =
        "digraph nba {\n".to_string()
        + &initial_state_arrow(&nba.initial_state)
        + &nba_edges_str
        + "}";
    fs::write(NBA_PATH, nba_output).unwrap();
    println!("Wrote NBA to {SIMPLIFIED_GBA_PATH}");



    // Simplified NBA
    println!("\n\n\n\n\nCreating simplified NBA");
    let simplified_nba = nba.simplify();
    
    let simplified_nba_edges = simplified_nba.delta.iter()
        .flat_map(|(source, targets)| {
            targets.iter().map(move |(symcon, target)| {
                format!("\"{}\" -> \"{}\" [label = \"{}\"]", source, target, symcon)
            })
        }).collect::<Vec<_>>();
    let simplified_nba_edges_str = simplified_nba_edges.join("\n");

    println!("Simplfied NBA initial state: {}", simplified_nba.initial_state);
    println!("Simplified NBA edges:");
    println!("{}", simplified_nba_edges_str);

    let simplified_nba_output =
        "digraph nba_simplified {\n".to_string()
        + &initial_state_arrow(&simplified_nba.initial_state)
        + &simplified_nba_edges_str
        + "}";
    fs::write(SIMPLIFIED_NBA_PATH, simplified_nba_output).unwrap();
    println!("Wrote simplified NBA to {SIMPLIFIED_GBA_PATH}");
}