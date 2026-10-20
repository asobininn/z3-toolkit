use std::{any::Any, fmt::Debug, sync::Arc};

use z3::ast::Bool;

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
}
