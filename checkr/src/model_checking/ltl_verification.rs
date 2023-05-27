use std::collections::BTreeMap;

use crate::{pg::ProgramGraph, ast::Target, concurrency::ParallelProgramGraph, util::traits::Add};

use super::{ltl_ast::LTL, ModelCheckMemory, vwaa::VWAA, gba::GBA, ba::BA, nested_dfs::{nested_dfs, LTLVerificationResult}, simplification::SimplifiableAutomaton};

pub fn verify_ltl(program_graph: &ParallelProgramGraph, ltl: LTL, initial_memory: &ModelCheckMemory, search_depth: usize) -> LTLVerificationResult {
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

pub fn zero_initialized_memory(pg: &ParallelProgramGraph, array_length: usize) -> ModelCheckMemory {
    let targets = pg.0.iter().flat_map(ProgramGraph::fv).collect::<Vec<_>>();

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

#[cfg(test)]
pub mod test {
    use crate::{parse::{parse_parallel_commands}, pg::Determinism, model_checking::{ltl_ast::parse_ltl}, concurrency::ParallelProgramGraph};

    use super::*;

    fn verify(program: &str, ltl: &str, det: Determinism) -> LTLVerificationResult {
        let pg = ParallelProgramGraph::new(det, &parse_parallel_commands(program).unwrap());
        verify_ltl(
            &pg,
            parse_ltl(ltl).unwrap(),
            &zero_initialized_memory(&pg, 10),
            100
        )
    }

    pub fn verify_satisfies(program: &str, ltl: &str) {
        assert_eq!(verify(program, ltl, Determinism::NonDeterministic), LTLVerificationResult::CycleNotFound)
    }

    pub fn verify_satisfies_det(program: &str, ltl: &str) {
        assert_eq!(verify(program, ltl, Determinism::Deterministic), LTLVerificationResult::CycleNotFound)
    }

    pub fn verify_not_satisfies(program: &str, ltl: &str) {
        match verify(program, ltl, Determinism::NonDeterministic) {
            LTLVerificationResult::CycleFound(_c) => {
                // println!("{:#?}", c);
            },
            _ => panic!(),
        }
    }

    pub fn verify_not_satisfies_det(program: &str, ltl: &str) {
        match verify(program, ltl, Determinism::Deterministic) {
            LTLVerificationResult::CycleFound(c) => {
                println!("{:#?}", c);
            },
            _ => panic!(),
        }
    }

    #[test]
    fn set_0_forever() {
        let program = "
        n := 0
        ";
        verify_satisfies(program, "[]{n >= 0}");
    }

    #[test]
    fn set_1_eventually() {
        let program = "
        n := 1
        ";
        verify_satisfies(program, "<>{n >= 1}");
    }

    #[test]
    fn set_below_0() {
        let program = "
        n := 0;
        n := -1
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

    #[test]
    fn set_1() {
        let program = "
        n := 1
        ";
        verify_satisfies(program, "{n = 0}");
    }

    #[test]
    fn set_1_false() {
        let program = "
        n := 1
        ";
        verify_not_satisfies(program, "{n = 1}");
    }

    #[test]
    fn set_1_next() {
        let program = "
        n := 1
        ";
        verify_satisfies(program, "(){n = 1}");
    }

    #[test]
    fn set_1_next_false() {
        let program = "
        n := 1
        ";
        verify_not_satisfies(program, "(){n = 2}");
    }

    #[test]
    fn skip_false() {
        let program = "
        skip
        ";
        verify_not_satisfies(program, "false");
    }

    #[test]
    fn skip_true() {
        let program = "
        skip
        ";
        verify_satisfies(program, "true");
    }

    #[test]
    fn nested_untils() {
        let program = "
        n := 1;
        n := 2;
        n := 3;
        n := 4;
        n := 5;
        n := 6
        ";
        verify_satisfies(program, "({n < 3} U {n = 3}) U ({n < 6} U {n = 6})");
    }

    #[test]
    fn nested_untils_false_left() {
        let program = "
        n := 1;
        n := 10;
        n := 3;
        n := 4;
        n := 5;
        n := 6
        ";
        verify_not_satisfies(program, "({n < 3} U {n = 3}) U ({n < 6} U {n = 6})");
    }

    #[test]
    fn nested_untils_false_right() {
        let program = "
        n := 1;
        n := 2;
        n := 3;
        n := 4;
        n := 10;
        n := 6
        ";
        verify_not_satisfies(program, "({n < 3} U {n = 3}) U ({n < 6} U {n = 6})");
    }

    #[test]
    fn next() {
        let program = "
        do true ->
            i := 1;
            i := 2
        od
        ";
        verify_satisfies(program, "[]({i = 1} -> (){i = 2})");
    }

    #[test]
    fn next_2() {
        let program = "
        do true ->
            i := 1;
            i := 2;
            i := 3
        od
        ";
        verify_satisfies(program, "[]({i = 1} -> ()(){i = 3})");
    }

    #[test]
    fn next_3() {
        let program = "
        do true ->
            i := 1;
            i := 2;
            i := 3
        od
        ";
        verify_not_satisfies(program, "[]({i = 1} -> ()(){i = 2})");
    }

    #[test]
    fn next_4() {
        let program = "
        do true ->
            if i = 0 -> i := 1
            [] i = 1 -> i := 0
            fi
        od
        ";
        verify_satisfies(program, "[]({i = 0} -> ()()(){i = 1})");
    }

    #[test]
    fn stuck() {
        let program = "
        n := 0;
        do false -> skip od
        ";
        verify_satisfies(program, "[]{n = 0}");
    }

    #[test]
    fn stuck_false() {
        let program = "
        n := 0;
        do false -> skip od
        ";
        verify_not_satisfies(program, "[]{n = 1}");
    }

    #[test]
    fn next_past_end() {
        let program = "
        n := 1
        ";
        verify_satisfies(program, "()(){n = 1}");
    }
}