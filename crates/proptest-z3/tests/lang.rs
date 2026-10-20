use std::collections::HashMap;

use proptest::prelude::*;
use proptest_z3::prelude::*;

const VAR: Space = Space::new("var");
const FUNC: Space = Space::new("func");

// ==== Generate ====================

fn program() -> impl Strategy<Value = Skeleton> {
    prop::collection::vec(stmt(), 1..8).prop_map(steps)
}

fn steps(stmts: impl IntoIterator<Item = Frag>) -> Skeleton {
    Skeleton::steps(stmts.into_iter().map(|s| frag![s, "\n"]))
}

fn stmt() -> impl Strategy<Value = Frag> {
    let leaf = prop_oneof![
        VAR.def().prop_map(|x| frag!["def ", x]),
        (VAR.def(), VAR.refer()).prop_map(|(x, y)| frag!["set ", x, " ", y]),
        VAR.refer().prop_map(|x| frag!["print ", x]),
        FUNC.def().prop_map(|f| frag!["fn ", f]),
        FUNC.refer().prop_map(|f| frag!["call ", f]),
    ];
    leaf.prop_recursive(3, 24, 4, |inner| {
        prop::collection::vec(inner, 0..4).prop_map(|body| {
            frag![
                "{ ",
                Frag::scope(body.into_iter().map(|s| frag![s, " "])),
                "}"
            ]
        })
    })
}

fn names() -> NameSpec {
    NameSpec::new()
        .space(VAR, "[a-z]{1,2}", 3)
        .space(FUNC, "[A-Z][a-z]?", 2)
}

// ==== Checker ====================

#[derive(Debug, Clone, Copy, PartialEq)]
enum Bug {
    Unbound,
}

fn the_mark(s: &Solved) -> (usize, Bug) {
    let [(hole, bug)] = s.marks::<Bug>()[..] else {
        panic!("one mark expected: {s:?}")
    };
    (s.span(hole).start, bug)
}

fn check(text: &str) -> Vec<usize> {
    let mut words =
        (text.split_whitespace()).map(|w| (w, w.as_ptr() as usize - text.as_ptr() as usize));
    let mut frames = vec![HashMap::new()];
    let mut undefined = vec![];
    while let Some((word, _)) = words.next() {
        let space = if matches!(word, "fn" | "call") {
            "func"
        } else {
            "var"
        };
        let mut name = || words.next().expect("a name after the keyword");
        let mut refer = |frames: &Vec<HashMap<_, bool>>, (name, at): (&str, usize)| {
            let state: Vec<bool> = frames
                .iter()
                .filter_map(|f| f.get(&(space, name)).copied())
                .collect();
            match (state.contains(&true), state.is_empty()) {
                (true, _) => {}
                (false, _) => undefined.push(at),
            }
        };
        match word {
            "{" => frames.push(HashMap::new()),
            "}" => drop(frames.pop()),
            "def" | "fn" => drop(frames.last_mut().unwrap().insert((space, name().0), true)),
            "set" => {
                let (x, y) = (name(), name());
                refer(&frames, y);
                frames.last_mut().unwrap().insert((space, x.0), true);
            }
            "print" | "call" => refer(&frames, name()),
            other => panic!("unknown keyword `{other}`"),
        }
    }
    undefined
}

// ==== Tests ====================

proptest! {
    /// Positive case: every reference points to a visible name.
    #[test]
    fn bound_has_no_erros(s in program().fill(names(), |binds| bound(binds, VAR) & bound(binds, FUNC))) {
        prop_assert_eq!(check(&s.text), vec![], "{:?}", s);
    }

    /// Negative case: exactly one reference is undefined, and the error is reported
    /// at the position of the marked hole.
    #[test]
    fn one_unbound_is_found_at_mark(s in program().fill(names(), |binds| constraint!(
        bound(binds, FUNC),
        exactly(1) { for i in binds.refs(VAR) {
            binds.mark(i, Bug::Unbound, !visible(binds, i))
        }},
    ))) {
        let (at, _) = the_mark(&s);
        prop_assert_eq!(check(&s.text), vec![at], "{:?}", s)
    }
}
