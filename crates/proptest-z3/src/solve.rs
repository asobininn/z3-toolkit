use std::{fmt::Debug, ops::Range};

use z3::{
    Model, Optimize,
    ast::{Bool, Int},
};
use z3_toolkit::prelude::*;

use crate::{Binds, Label, Names, Role, Skeleton, Space, frag::Piece, label::Tag};

/// How the preferred names (hints) are treated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Fill {
    #[default]
    /// Maximizes the number of hints kept. Faithful, but slow with many holes.
    Optimal,
}

/// Index of a hole in a result (in text order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Hole(pub usize);

/// A single hole after filling.
#[derive(Debug, Clone)]
pub struct FilledHole {
    pub space: Space,
    pub role: Role,
    pub name: String,
    /// Position in the output text (in bytes).
    pub span: Range<usize>,
}

#[derive(Clone)]
pub struct Solved {
    pub text: String,
    /// Holes, in text order.
    holes: Vec<FilledHole>,
    /// Holes, in evaluation order.
    events: Vec<Hole>,
    /// Labels whose conditions held.
    hits: Vec<(Option<Hole>, Tag)>,
}

impl Solved {
    /// Holes (in text order).
    pub fn holes(&self) -> impl Iterator<Item = (Hole, &FilledHole)> {
        self.holes.iter().enumerate().map(|(i, h)| (Hole(i), h))
    }

    pub fn hole(&self, hole: Hole) -> &FilledHole {
        &self.holes[hole.0]
    }

    /// Position of a hole (in bytes).
    pub fn span(&self, hole: Hole) -> Range<usize> {
        self.holes[hole.0].span.clone()
    }

    /// The name filled into a hole.
    pub fn name(&self, hole: Hole) -> &str {
        &self.holes[hole.0].name
    }

    /// Holes in evalution order.
    pub fn events(&self) -> &[Hole] {
        &self.events
    }

    pub fn marks<L: Label>(&self) -> Vec<(Hole, L)> {
        self.hits
            .iter()
            .filter_map(|(h, t)| Some(((*h)?, t.get()?)))
            .collect()
    }

    /// The text with line number.
    pub fn numbered(&self) -> String {
        (self.text.lines().enumerate())
            .map(|(i, l)| format!("{i:>3}| {l}\n"))
            .collect()
    }
}

impl Debug for Solved {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Solved{{")?;
        write!(f, "{}", self.numbered())?;
        let mut marks = vec![];
        for (h, t) in &self.hits {
            if let Some(h) = h {
                marks.push(format!(
                    "{} @ {:?} `{}`",
                    t.name,
                    self.span(*h),
                    self.name(*h)
                ));
            }
        }
        if !marks.is_empty() {
            writeln!(f, " marks: {}", marks.join(", "))?;
        }
        write!(f, "}}")
    }
}

impl Skeleton {
    /// Fills the holes.
    pub fn solve(
        &self,
        names: &Names,
        fill: Fill,
        rule: impl Fn(&Binds) -> Bool,
    ) -> Result<Solved, Failure> {
        let frag = &self.0;
        let binds = Binds::new(frag);
        let count = |space: Space| {
            let n = names.get(space).len();
            assert!(n > 0, "no names for space `{}`", space.name());
            n
        };
        let mut hard: Vec<Bool> = (0..binds.len())
            .map(|i| constraint!(binds.name[i] in 0..count(binds.space[i])))
            .collect();
        hard.push(rule(&binds));
        let wishes: Vec<Bool> = (0..binds.len())
            .map(|i| {
                let hint = frag.holes[binds.hole[i]].hint.index(count(binds.space[i]));
                constraint!(binds.name[i] == hint)
            })
            .collect();
        let model = match fill {
            Fill::Optimal => optimal(&hard, &wishes)?,
        };
        // Number in evalution order → Name of the hole in text order
        let picked: Vec<usize> = model.get_as(&binds.name)?;
        let mut by_hole = vec![0; frag.holes.len()];
        for (i, &v) in picked.iter().enumerate() {
            by_hole[binds.hole[i]] = v;
        }
        // Insert into the text in order, recording the position of each hole
        let mut text = String::new();
        let mut holes = Vec::with_capacity(frag.holes.len());
        for piece in &frag.pieces {
            match piece {
                Piece::Text(s) => text.push_str(s),
                Piece::Hole(h) => {
                    let info = &frag.holes[*h];
                    let name = names.get(info.space)[by_hole[*h]].clone();
                    let start = text.len();
                    text.push_str(&name);
                    holes.push(FilledHole {
                        space: info.space,
                        role: info.role,
                        name,
                        span: start..text.len(),
                    });
                }
            }
        }
        let mut hits = vec![];
        for (at, tag, cond) in &binds.records.borrow().labels {
            let value = (model.eval(cond, true).and_then(|b| b.as_bool())).ok_or_else(|| {
                Failure::Unknown {
                    reason: format!("cannot evalute label {}", tag.name),
                }
            })?;
            if value {
                hits.push((at.map(|i| Hole(binds.hole[i])), tag.clone()));
            }
        }
        Ok(Solved {
            text,
            holes,
            events: binds.hole.iter().map(|&h| Hole(h)).collect(),
            hits,
        })
    }
}

fn assert_hard(engine: &impl Engine, hard: &[Bool]) {
    engine.assert_all(hard);
}
fn optimal(hard: &[Bool], wishes: &[Bool]) -> Result<Model, Failure> {
    let opt = Optimize::new();
    assert_hard(&opt, hard);
    let agree: Int = sum(wishes);
    opt.maximize(&agree);
    opt.solve()
}
