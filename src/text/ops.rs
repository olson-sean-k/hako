use std::convert::Infallible;

use crate::text::{BlockLayout, BlockText, LinearLayout, Morpheme, Narrow, Wide};

pub use Layer::{Back, Front};

// NOTE: Previous implementations provided some notion of repetition that "multiplied" text by some
//       natural number by allocating and returning the complete repeated text. Instead, use
//       `graphemes` and `morphemes` with the `cycle` combinator and pull single morphemes at a
//       time to avoid excessive allocation. `Repeat::repeat(&self, n: usize) -> String` is a
//       pretty scary interface! Don't expose something like that.

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Congruent<L, R> {
    left: L,
    right: R,
}

impl<L, R> Congruent<L, R> {
    pub fn try_from_blocks(left: L, right: R) -> Result<Self, (L, R)>
    where
        L: BlockLayout,
        R: BlockLayout<Height = L::Height>,
    {
        if left.ascii_line_break_bounds() == right.ascii_line_break_bounds() {
            Ok(Congruent { left, right })
        }
        else {
            Err((left, right))
        }
    }
}

pub trait Truncate: LinearLayout {
    fn truncate(&mut self, max: usize) -> usize;
}

pub trait Extend: LinearLayout {
    fn extend<'t, I>(&'t mut self, morphemes: I) -> usize
    where
        I: IntoIterator<Item = Self::Morpheme<'t>>;

    fn fill<'t, I>(&'t mut self, min: usize, morphemes: I) -> usize
    where
        I: IntoIterator<Item = Self::Morpheme<'t>>,
        I::IntoIter: Clone,
    {
        let mut width = self.width();
        self.extend(morphemes.into_iter().cycle().take_while(|morpheme| {
            width = width
                .checked_add(morpheme.width().into())
                .expect("overflow extending text");
            width < min
        }))
    }
}

// NOTE: This is similar to `Extend`, but is closed over text types: it accepts two `T`s and
//       outputs their concatenation (also a `T`). This is useful, as types like `Line` can support
//       the composition of text types using `&str` representations (because `Line` can append
//       `Segment`s).
//
//       Also, unlike `Extend`, this trait may avoid copies, as `Extend` requires reading and
//       copying morphemes from the RHS. `Append` may move or consolidate buffers.
pub trait Append: LinearLayout {
    fn append(self, rhs: Self) -> Self;
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Layer<F, B> {
    Front(F),
    Back(B),
}

pub type Selection = Layer<(), ()>;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Layered<F, B> {
    front: F,
    back: B,
}

pub type Fill<'t> = (Narrow<'t>, Narrow<'t>);

pub type Contraction<'t> = Fill<'t>;

#[derive(Clone, Debug, PartialEq)]
pub enum Expansion<'t> {
    Fill(Fill<'t>),
    Wide(Wide<'t>),
}

#[derive(Debug)]
pub struct Receipt(());

impl Receipt {
    fn new() -> Self {
        Receipt(())
    }
}

#[derive(Debug)]
pub struct BlendBy<'o, IF, IB, Output> {
    output: &'o mut Output,
    input: Layered<IF, IB>,
}

impl<'o, IF, IB, Output> BlendBy<'o, IF, IB, Output> {
    pub fn blend<F>(self, f: F) -> Receipt
    where
        F: FnOnce(Layered<IF, IB>) -> Output,
    {
        let BlendBy { output, input } = self;
        *output = f(input);
        Receipt::new()
    }
}

pub type Select<'o, M> = BlendBy<'o, M, M, Selection>;
pub type Contract<'t, 'o> = BlendBy<'o, Wide<'t>, Fill<'t>, Contraction<'t>>;
pub type Expand<'t, 'o> = BlendBy<'o, Fill<'t>, Wide<'t>, Expansion<'t>>;

#[derive(Debug)]
pub enum Blend<'t, 'o, M>
where
    M: 't,
{
    Select(Select<'o, M>),
    Contract(Contract<'t, 'o>),
    Expand(Expand<'t, 'o>),
}

pub trait Overlay: BlockText {
    type Blend<'o>;
    type Output: BlockText;

    // TODO: The name `Output` in `BlockText` and friends is bad. Simplify this once it is renamed!
    fn overlay_with<F>(self, f: F) -> <Self as Overlay>::Output
    where
        F: FnMut(Self::Blend<'_>) -> Receipt;
}

pub trait TryOverlay: BlockText {
    type Error;
    type Blend<'o>;
    type Output: BlockText;

    // TODO: The name `Output` in `BlockText` and friends is bad. Simplify this once it is renamed!
    fn try_overlay_with<F>(self, f: F) -> Result<<Self as TryOverlay>::Output, Self::Error>
    where
        F: FnMut(Self::Blend<'_>) -> Receipt;
}

impl<T> TryOverlay for T
where
    T: Overlay,
{
    type Error = Infallible;
    type Blend<'o> = T::Blend<'o>;
    // TODO: The name `Output` in `BlockText` and friends is bad. Simplify this once it is renamed!
    type Output = <T as Overlay>::Output;

    // TODO: The name `Output` in `BlockText` and friends is bad. Simplify this once it is renamed!
    fn try_overlay_with<F>(self, f: F) -> Result<<Self as TryOverlay>::Output, Self::Error>
    where
        F: FnMut(Self::Blend<'_>) -> Receipt,
    {
        Ok(self.overlay_with(f))
    }
}

// Overlay Conflicts
//
// wide over narrow
// ================
// [ a  ]
// [b][c]
//
// narrow over wide
// ===============
// [a][b]
// [ c  ]
//
// narrow over flex
// ================
// [a][b]
// [ c  ]
//
// flex over flex
// ==============
// [ a  ]
// [b][c]
//
// [a][b]
// [ c  ]
//
// these could run on until the end of the text; it's likely that this can't be elegantly resolved
// and so `TryOverlay` should just fail if this occurs.
//
// [ a  ]
// [b][ c  ]
//
// [a][ b  ]
// [ c  ]
