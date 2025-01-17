use std::num::NonZeroUsize;

use crate::text::BlockTextProjection;

// Count of columns and rows of some text. This is somewhat general, but generally considers line
// breaks.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BoundingBox<H> {
    pub width: usize,
    pub height: H,
}

pub trait BlockGeometry: BlockTextProjection {
    type Height: Copy + Eq + Into<usize> + Ord;

    // It is important to keep these kinds of bounds distinct from `Unicode::width`, `str::len`,
    // etc.! These functions should **always** consider the complete text as a sum. Here, ASCII
    // line breaks are used to consider the structure of the text, and so the width bound is a
    // maximum by line. Imagine if `str::len` or some other `len` function did this: it would be
    // quite confusing. Do not conflate these concepts in APIs.
    fn ascii_line_break_bounds(&self) -> BoundingBox<Self::Height>;
}

pub trait LinearGeometry: BlockTextProjection {
    fn width(&self) -> usize;
}

// `LinearGeometry` is more specific than `BlockGeometry`. These traits are not mutually exclusive.
impl<T> BlockGeometry for T
where
    T: LinearGeometry,
{
    type Height = NonZeroUsize;

    fn ascii_line_break_bounds(&self) -> BoundingBox<Self::Height> {
        BoundingBox {
            width: self.width(),
            height: NonZeroUsize::MIN,
        }
    }
}
