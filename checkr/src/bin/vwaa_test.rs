use std::fs;

use checkr::{model_checking::{ltl::LTL, vwaa::{VWAA, LTLConjunction}, gba::{GBA, ltlset_string, GBATransition}, simplification::SimplifiableAutomaton, ba::BA}, ast::{BExpr, AExpr, RelOp}};


fn main() {
    let formula = LTL::Not(
        Box::new(LTL::Implies(
            Box::new(LTL::Forever(
                Box::new(LTL::Eventually(
                    Box::new(LTL::Atomic(BExpr::Rel(
                        AExpr::Number(5), 
                        RelOp::Ge, 
                        AExpr::Number(4)
                    )))
                ))
            )),
            Box::new(LTL::Forever(
                Box::new(LTL::Implies(
                    Box::new(LTL::Atomic(
                        BExpr::Rel(
                            AExpr::Number(10), RelOp::Ge, AExpr::Number(9)
                        )
                    )),
                    Box::new(LTL::Eventually(Box::new(LTL::Atomic(BExpr::Rel(AExpr::Number(11), RelOp::Ge, AExpr::Number(10))))))
                ))
            ))
        ))
    );

    let reduced = dbg!(formula.reduced());
    let nn = dbg!(reduced.to_negative_normal());
    println!("\n\n\n\n\nCreating VWAA");
    let vwaa = dbg!(VWAA::from_ltl(&nn));

    let vwaa_edges = vwaa.delta().iter()
        .flat_map(|(source, targets)| {
            targets.iter().flat_map(move |(symcon, ltlcon)| {
                match ltlcon {
                    LTLConjunction::TT => vec! [format!("\"{}\" -> \"{}\" [label = \"{}\"]", source, "tt", symcon)],
                    LTLConjunction::Conjunction(ltlcon) => ltlcon.iter().map(move |ltl| {
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
    fs::write("graphviz_output/vwaa5.dot", graphviz_output).unwrap();


    // GBA
    println!("\n\n\n\n\nCreating GBA");
    let gba = GBA::from_vwaa(vwaa);

    println!("{}", gba.states.iter().map(|e| ltlset_string(e)).collect::<Vec<_>>().join(",\n"));

    let gba_edges = gba.delta.iter()
        .flat_map(|(source, targets)| {
            targets.iter().map(move |(symcon, target)| {
                format!("\"{}\" -> \"{}\" [label = \"{}\"]", ltlset_string(source), ltlset_string(target), symcon)
            })
        }).collect::<Vec<_>>();
    let gba_edges_str = gba_edges.join("\n");

    println!("GBA edges:");
    println!("{}", gba_edges_str);

    let gba_output =
        "digraph gba {\n".to_string()
        + &gba_edges_str
        + "}";
    fs::write("graphviz_output/gba3.dot", gba_output).unwrap();


    // Simplified GBA
    println!("\n\n\n\n\nCreating simplified GBA");
    let simplified_gba = gba.simplify();

    println!("{}", simplified_gba.states.iter().map(|e| ltlset_string(e)).collect::<Vec<_>>().join(",\n"));

    let simplified_gba_edges = simplified_gba.delta.iter()
        .flat_map(|(source, targets)| {
            targets.iter().map(move |(symcon, target)| {
                format!("\"{}\" -> \"{}\" [label = \"{}\"]", ltlset_string(source), ltlset_string(target), symcon)
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
            println!("source: {}, action: {}, target: {}", ltlset_string(source), action, ltlset_string(target));
        }
    }

    let simplified_gba_output =
        "digraph simplified_gba {\n".to_string()
        + &simplified_gba_edges_str
        + "}";
    fs::write("graphviz_output/gba_simplified2.dot", simplified_gba_output).unwrap();


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

    let ba_output =
        "digraph ba {\n".to_string()
        + &ba_edges_str
        + "}";
    fs::write("graphviz_output/ba.dot", ba_output).unwrap();



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

    let simplified_ba_output =
        "digraph ba_simplified {\n".to_string()
        + &simplified_ba_edges_str
        + "}";
    fs::write("graphviz_output/ba_simplified2.dot", simplified_ba_output).unwrap();



}