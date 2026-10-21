use std::{any::Any, cell::RefCell, collections::BTreeMap, fmt::Debug, io::Write, sync::Arc};

use z3::ast::Bool;
use z3_toolkit::Failure;

/// Types usable as labels: anything `Copy` with `debug` and equality
pub trait Label: Copy + Debug + PartialEq + Send + Sync + 'static {}
impl<T: Copy + Debug + PartialEq + Send + Sync + 'static> Label for T {}

#[derive(Clone)]
pub(crate) struct Tag {
    kind: Arc<dyn Any + Send + Sync>,
    pub name: String,
}

impl Tag {
    pub fn new<L: Label>(kind: L) -> Self {
        let name = match (&kind as &dyn Any).downcast_ref::<&'static str>() {
            Some(s) => s.to_string(),
            None => format!("{kind:?}"),
        };
        Self {
            kind: Arc::new(kind),
            name,
        }
    }

    pub fn get<L: Label>(&self) -> Option<L> {
        self.kind.downcast_ref::<L>().copied()
    }
}

#[derive(Default)]
pub(crate) struct Records {
    pub labels: Vec<(Option<usize>, Tag, Bool)>,
    pub required: Vec<(String, Bool)>,
}

/// Asks this conditin only when all `guards` hold (self ∧ gards).
pub trait Given {
    fn given(self, guard: impl IntoIterator<Item = Bool>) -> Bool;
}

impl Given for Bool {
    fn given(self, guard: impl IntoIterator<Item = Bool>) -> Bool {
        let all: Vec<Bool> = std::iter::once(self).chain(guard).collect();
        Bool::and(&all)
    }
}

/// Statistics for a single label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelStat {
    pub label: String,
    pub declared: usize,
    pub occurred: usize,
}

#[derive(Default)]
struct Stats {
    // (label name, (recorded case, case where it was true))
    labels: BTreeMap<String, (usize, usize)>,
    // (unsat core name, count)
    rejections: BTreeMap<Vec<String>, usize>,
}

thread_local! {
    static STATS: RefCell<Stats> = RefCell::default();
}

fn with_stats<T>(f: impl FnOnce(&mut Stats) -> T) -> T {
    STATS.with(|s| f(&mut s.borrow_mut()))
}

pub(crate) fn count_case(declared: &BTreeMap<String, bool>) {
    with_stats(|s| {
        for (name, &hit) in declared {
            let (decl, occ) = s.labels.entry(name.clone()).or_default();
            *decl += 1;
            *occ += hit as usize;
        }
    })
}

pub(crate) fn count_rejection(failure: &Failure) {
    let key = match failure {
        Failure::Unsat { core } => {
            let mut core = core.clone();
            core.sort();
            core
        }
        other => vec![format!("{other}")],
    };
    with_stats(|s| *s.rejections.entry(key).or_default() += 1);
}

fn to_stats(labels: &BTreeMap<String, (usize, usize)>) -> Vec<LabelStat> {
    (labels.iter())
        .map(|(label, &(declared, occurred))| LabelStat {
            label: label.clone(),
            declared,
            occurred,
        })
        .collect()
}

/// Per-label statistics of the caases generated so far on this thread.
pub fn label_stats() -> Vec<LabelStat> {
    with_stats(|s| to_stats(&s.labels))
}

/// Takes [`label_stats`] and resets the statistics.
pub fn take_label_stats() -> Vec<LabelStat> {
    with_stats(|s| to_stats(&std::mem::take(&mut s.labels)))
}

/// Takes the rejection reasons and their counts so far, and resets them.
pub fn take_rejections() -> Vec<(Vec<String>, usize)> {
    with_stats(|s| std::mem::take(&mut s.rejections).into_iter().collect())
}

impl Drop for Stats {
    fn drop(&mut self) {
        let thread = std::thread::current();
        let mut err = std::io::stderr().lock();
        for s in to_stats(&self.labels)
            .into_iter()
            .filter(|s| s.occurred == 0)
        {
            let _ = writeln!(
                err,
                "proptest-z3 warnings: label `{}` never occurred in {} cases (test `{}`);\
             is it implied by another violation? consider `given`",
                s.label,
                s.declared,
                thread.name().unwrap_or("?")
            );
        }
    }
}
