use std::fmt::Debug;
use std::num::NonZeroUsize;

use crate::text::morphology::Grapheme;
use crate::text::{BlockText, Indexed};

// Count of columns and rows of some text. This is somewhat general, but generally considers line
// breaks.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BoundingBox<H> {
    pub width: usize,
    pub height: H,
}

// This is **not** mutually exclusive with `BlockLayout`.
pub trait LinearGeometry: BlockText {
    fn width(&self) -> usize;
}

pub trait BlockGeometry: BlockText {
    type Height: Copy + Eq + Into<usize> + Ord;

    // It is important to keep these kinds of bounds distinct from `Unicode::width`, `str::len`,
    // etc.! These functions should **always** consider the complete text as a sum. Here, ASCII
    // line breaks are used to consider the structure of the text, and so the width bound is a
    // maximum by line. Imagine if `str::len` or some other `len` function did this: it would be
    // quite confusing. Do not conflate these concepts in APIs.
    fn ascii_line_break_bounds(&self) -> BoundingBox<Self::Height>;
}

// This makes interpreting linear text types as blocks explicit.
#[derive(Debug)]
#[repr(transparent)]
pub(in crate::text) struct AsBlockGeometry<'t, T>(pub &'t T);

impl<'t, T> BlockGeometry for AsBlockGeometry<'t, T>
where
    T: LinearGeometry,
{
    type Height = NonZeroUsize;

    fn ascii_line_break_bounds(&self) -> BoundingBox<Self::Height> {
        BoundingBox {
            width: self.0.width(),
            height: NonZeroUsize::MIN,
        }
    }
}

impl<'t, T> BlockText for AsBlockGeometry<'t, T>
where
    T: BlockText,
{
    type RawText = <T as BlockText>::RawText;
    type Morpheme<'m>
        = T::Morpheme<'m>
    where
        Self: 'm;
    type Index = T::Index;

    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<Self::Index, Grapheme<'_>>> {
        self.0.graphemes()
    }
}
