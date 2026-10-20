use std::{collections::BTreeMap, fmt::Debug};

use proptest::{
    prelude::prop,
    strategy::{BoxedStrategy, Strategy, ValueTree},
};

use crate::Space;

/// Name candidates per namespace
#[derive(Debug, Clone, Default)]
pub struct Names {
    spaces: BTreeMap<Space, Vec<String>>,
}

impl Names {
    pub fn get(&self, space: Space) -> &[String] {
        self.spaces.get(&space).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Sets candidates by hand (for tests and exaples).
    pub fn with(
        mut self,
        space: Space,
        names: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.spaces
            .insert(space, names.into_iter().map(Into::into).collect());
        self
    }
}

/// A strategy that generates name candidates. Add one namespace at a time.
#[derive(Clone, Default)]
pub struct NameSpec {
    spaces: BTreeMap<Space, BoxedStrategy<Vec<String>>>,
}

impl NameSpec {
    pub fn new() -> Self {
        Self::default()
    }

    /// `count` distinct names matcing `regex`.
    pub fn space(self, space: Space, regex: &str, count: usize) -> Self {
        let one = proptest::string::string_regex(regex).expect("invalid regex for names");
        let names = prop::collection::btree_set(one, count)
            .prop_map(|set| set.into_iter().collect::<Vec<_>>())
            .boxed();
        self.space_with(space, names)
    }

    /// Passes a strategy that generate the candidates directly. It must return a sequence without duplicates.
    pub fn space_with(mut self, space: Space, names: BoxedStrategy<Vec<String>>) -> Self {
        let prev = self.spaces.insert(space, names);
        assert!(prev.is_none(), "space `{}` registered twice", space.name());
        self
    }
}

impl Debug for NameSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NameSpec")
            .field("spaces", &self.spaces)
            .finish_non_exhaustive()
    }
}

impl Strategy for NameSpec {
    type Tree = Box<dyn ValueTree<Value = Names>>;
    type Value = Names;

    fn new_tree(
        &self,
        runner: &mut prop::test_runner::TestRunner,
    ) -> prop::strategy::NewTree<Self> {
        let keys: Vec<Space> = self.spaces.keys().copied().collect();
        let values: Vec<_> = self.spaces.values().cloned().collect();
        values
            .prop_map(move |all| Names {
                spaces: keys.iter().copied().zip(all).collect(),
            })
            .boxed()
            .new_tree(runner)
    }
}
