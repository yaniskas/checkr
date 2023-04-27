use crate::pg::ProgramGraph;

use super::{ltl_ast::LTL, ModelCheckMemory, vwaa::VWAA, gba::GBA, ba::BA, nested_dfs::{nested_dfs, LTLVerificationResult}, simplification::SimplifiableAutomaton};

pub fn verify_ltl(program_graph: &ProgramGraph, ltl: LTL, initial_memory: &ModelCheckMemory, search_depth: usize) -> LTLVerificationResult {
    let formula = LTL::Not(Box::new(ltl));
    let reduced = formula.reduced();
    let nn = reduced.to_negative_normal();

    let vwaa = VWAA::from_ltl(&nn);
    let gba = GBA::from_vwaa(vwaa);
    let simplified_gba = gba.simplify();
    let ba = BA::from_gba(simplified_gba);
    let simplified_ba = ba.simplify();

    nested_dfs(program_graph, &simplified_ba, initial_memory, search_depth)
}

#[cfg(test)]
mod test {
    use std::collections::{HashMap, BTreeMap};

    use crate::{parse::parse_commands, pg::Determinism, ast::Target, model_checking::{traits::Add, ltl_ast::parse_ltl}};

    use super::*;

    fn zero_initialized_memory(pg: &ProgramGraph, array_length: usize) -> ModelCheckMemory {
        let targets = pg.fv();

        let empty_memory = ModelCheckMemory {
            variables: BTreeMap::new(),
            arrays: BTreeMap::new(),
        };

        targets.into_iter().fold(empty_memory, |acc, e| {
            match e {
                Target::Variable(v) => ModelCheckMemory {
                    variables: acc.variables.add((v, 0)),
                    ..acc
                },
                Target::Array(a, _) => {
                    let mut vec = Vec::new();
                    for _ in 0..array_length {vec.push(0)}
                    ModelCheckMemory {
                        arrays: acc.arrays.add((a, vec)),
                        ..acc
                    }
                }
            }
        })
    }

    fn verify(program: &str, ltl: &str) -> LTLVerificationResult {
        let pg = ProgramGraph::new(Determinism::NonDeterministic, &parse_commands(program).unwrap());
        verify_ltl(
            &pg,
            parse_ltl(ltl).unwrap(),
            &zero_initialized_memory(&pg, 10),
            100
        )
    }

    fn verify_satisfies(program: &str, ltl: &str) {
        assert_eq!(verify(program, ltl), LTLVerificationResult::CycleNotFound)
    }

    fn verify_not_satisfies(program: &str, ltl: &str) {
        match verify(program, ltl) {
            LTLVerificationResult::CycleFound(_) => {},
            _ => panic!(),
        }
    }

    #[test]
    fn set_0_forever() {
        let program = "
        n := 0;
        ";
        verify_satisfies(program, "[]{n >= 0}");
    }

    #[test]
    fn set_1_eventually() {
        let program = "
        n := 1;
        ";
        verify_satisfies(program, "<>{n >= 1}");
    }

    // #[test]
    // fn set_1() {
    //     let program = "
    //     n := 1;
    //     ";
    //     verify_satisfies(program, "{n = 1}");
    // }

    #[test]
    fn set_below_0() {
        let program = "
        n := 0;
        n := -1;
        ";
        verify_not_satisfies(program, "[]{n >= 0}");
    }

    #[test]
    fn nd_loop_below_0() {
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
        verify_not_satisfies(program, "[]{n >= 0}");
    }

    #[test]
    fn loop_around() {
        let program = "
        i := 0;
        do i < 6 ->
            i := i + 1
        [] i = 6 ->
            i := 0
        od
        ";
        verify_satisfies(program, "[]<>{i = 5}");
    }

    #[test]
    fn loop_above() {
        let program = "
        i := 0;
        do i < 6 ->
            i := i + 1
        [] i = 6 ->
            i := 1
        od
        ";
        verify_not_satisfies(program, "[]<>{i = 0}");
    }

    #[test]
    fn loop_above_sat() {
        let program = "
        i := 0;
        do i < 6 ->
            i := i + 1
        [] i = 6 ->
            i := 1
        od
        ";
        verify_satisfies(program, "<>[]{i > 0}");
    }

    #[test]
    fn loop_switch() {
        let program = "
        i := 10;
        do true ->
            do i < 20 ->
                i := i + 1
            od;
            i := 10
        [] true ->
            do i > 0 ->
                i := i - 1
            od;
            i := 10
        od
        ";
        verify_satisfies(program, "[]({i = 11} -> <> {i = 20})");
    }

    #[test]
    fn loop_switch_false() {
        let program = "
        i := 10;
        do true ->
            do i < 20 ->
                i := i + 1
            od;
            i := 10
        [] true ->
            do i > 0 ->
                i := i - 1
            od;
            i := 10
        od
        ";
        verify_not_satisfies(program, "[]({i = 10} -> <>{i = 20})");
    }
}