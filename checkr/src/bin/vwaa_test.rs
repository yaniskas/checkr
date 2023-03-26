use std::fs;

use checkr::{model_checking::{ltl::LTL, vwaa::VWAA}, ast::{BExpr, AExpr, RelOp}};



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
    let vwaa = dbg!(VWAA::from_ltl(&nn));

    let graphviz_edges = vwaa.delta().iter()
        .flat_map(|(source, targets)| {
            targets.iter().flat_map(move |(symcon, ltlcon)| {
                ltlcon.iter().map(move |ltl| {
                    format!("\"{}\" -> \"{}\" [label = \"{}\"]", source, ltl, symcon)
                })
            })
        }).collect::<Vec<_>>();
    let graphviz_edges_str = graphviz_edges.join("\n");

    println!("VWAA edges:");
    println!("{}", graphviz_edges_str);

    let graphviz_output =
        "digraph vwaa {\n".to_string()
        + &graphviz_edges_str
        + "}";
    fs::write("graphviz_output/vwaa2.dot", graphviz_output).unwrap();
}