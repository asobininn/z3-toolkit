//! Constants the toolkit makes for labels.
//!
//! - `"name": e` (unsat core): a tracker `z3-toolkit:name`, asserted with `e` at the top level.
//! - `"name" @ e` (display): `ite(z3-toolkit@:name, e, e)`. Its value is always `e`;
//!   the constant only marks the spot, so [`ModelExt::labels`](crate::ModelExt::labels) can find it.

use std::{
    collections::HashSet,
    sync::{Mutex, OnceLock},
};

use z3::{
    DeclKind, Model,
    ast::{Ast, Bool, Dynamic},
};

use crate::{ReadError, Readback};

pub(crate) const CORE_PREFIX: &str = "z3-toolkit:";
pub(crate) const TAG_PREFIX: &str = "z3-toolkit@";

/// Constants the toolkit made itself; `solve_all` must not block on them.
pub(crate) fn is_internal(name: &str) -> bool {
    name.starts_with(CORE_PREFIX) || name.starts_with(TAG_PREFIX)
}

/// Names seen by [`tag`], so readback can return the original `&'static str`.
fn names() -> &'static Mutex<HashSet<&'static str>> {
    static NAMES: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();
    NAMES.get_or_init(Default::default)
}

/// Expansion target of `"name" @ e`.
pub fn tag(name: &'static str, e: &Bool) -> Bool {
    names().lock().unwrap().insert(name);
    Bool::new_const(format!("{TAG_PREFIX}{name}")).ite(e, e)
}

/// `(name, e)` if `node` is `ite(z3-toolkit@:name, e, e)`.
fn as_tag(node: &Dynamic) -> Option<(&'static str, Bool)> {
    let decl = node.safe_decl().ok()?;
    if decl.kind() != DeclKind::Ite {
        return None;
    }
    let [cond, then, _] = <[Dynamic; 3]>::try_from(node.children()).ok()?;
    if !cond.is_const() {
        return None;
    }
    let full = cond.decl().name();
    let name = full.strip_prefix(TAG_PREFIX)?;
    let name = *names().lock().unwrap().get(name)?;
    Some((name, then.as_bool()?))
}

/// Labels under `roots` whose expression is true in `model`, in order of first appearance.
pub(crate) fn true_tags(model: &Model, roots: &[Dynamic]) -> Result<Vec<&'static str>, ReadError> {
    let mut seen = HashSet::new();
    let mut stack: Vec<Dynamic> = roots.iter().rev().cloned().collect();
    let mut found = Vec::new();
    // traversing an AST using DFS
    while let Some(node) = stack.pop() {
        if !node.is_app() || !seen.insert(node.get_z3_ast()) {
            continue;
        }
        if let Some((name, e)) = as_tag(&node)
            && !found.contains(&name)
            && e.read(model)?
        {
            found.push(name);
        }
        stack.extend(node.children().into_iter().rev());
    }
    Ok(found)
}

/// Shapes [`ModelExt::labels`](crate::ModelExt::labels) can search: `Bool`, and arrays, `Vec` and slices of it.
pub trait Labeled {
    fn roots(&self) -> Vec<Dynamic>;
}

impl Labeled for Bool {
    fn roots(&self) -> Vec<Dynamic> {
        vec![Dynamic::from_ast(self)]
    }
}

impl<T: Labeled + ?Sized> Labeled for &T {
    fn roots(&self) -> Vec<Dynamic> {
        (**self).roots()
    }
}

impl<T: Labeled> Labeled for [T] {
    fn roots(&self) -> Vec<Dynamic> {
        self.iter().flat_map(T::roots).collect()
    }
}

impl<T: Labeled, const N: usize> Labeled for [T; N] {
    fn roots(&self) -> Vec<Dynamic> {
        self.as_slice().roots()
    }
}

impl<T: Labeled> Labeled for Vec<T> {
    fn roots(&self) -> Vec<Dynamic> {
        self.as_slice().roots()
    }
}
