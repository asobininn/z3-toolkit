use std::collections::HashSet;

use proptest::prelude::*;
use proptest_z3::prelude::*;

const VAR: Space = Space::new("var");

// ==== Generate ====================

fn program() -> impl Strategy<Value = Skeleton> {
    prop::collection::vec(stmt(), 1..8).prop_map(steps)
}

fn steps(stmts: impl IntoIterator<Item = Frag>) -> Skeleton {
    Skeleton::steps(stmts.into_iter().map(|s| frag![s, "\n"]))
}

fn stmt() -> impl Strategy<Value = Frag> {
    prop_oneof![
        VAR.def().prop_map(|x| frag!["def ", x]),
        (VAR.def(), VAR.refer()).prop_map(|(x, y)| frag!["set ", x, " ", y]),
        VAR.refer().prop_map(|x| frag!["print ", x]),
    ]
}

fn names() -> NameSpec {
    NameSpec::new().space(VAR, "[a-z]{1,2}", 3)
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
    let mut defined = HashSet::new();
    let mut undefined = vec![];
    while let Some((keyword, _)) = words.next() {
        let mut name = || words.next().expect("a name after the keyword");
        let mut refer = |(y, at): (&str, usize), defined: &HashSet<&str>| {
            if !defined.contains(y) {
                undefined.push(at);
            }
        };
        match keyword {
            "def" => {
                defined.insert(name().0);
            }
            "set" => {
                let (x, y) = (name(), name());
                refer(y, &defined); // 右辺を先に見る
                defined.insert(x.0);
            }
            "print" => refer(name(), &defined),
            other => panic!("unknown keyword `{other}`"),
        }
    }
    undefined
}

// ==== Tests ====================

proptest! {
    /// Positive case: every reference points to a visible name.
    #[test]
    fn bound_has_no_erros(s in program().fill(names(), |bind| bound(bind, VAR))) {
        prop_assert_eq!(check(&s.text), vec![], "{:?}", s);
    }

    /// Negative case: exactly one reference is undefined, and the error is reported
    /// at the position of the marked hole.
    #[test]
    fn one_unbound_is_found_at_mark(s in program().fill(names(), |binds| constraint!(
        exactly(1) { for i in binds.refs(VAR) {
            binds.mark(i, Bug::Unbound, !visible(binds, i))
        }},
    ))) {
        let (at, _) = the_mark(&s);
        prop_assert_eq!(check(&s.text), vec![at], "{:?}", s)
    }
}
