use std::convert::Infallible;
use std::marker::PhantomData;

use crate::text::{
    Annex11, BlockLayout, Encoded, Grapheme, LinearLayout, Morpheme, Narrow, Unicode, Wide,
};

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
    pub fn try_from_block(left: L, right: R) -> Result<Self, (L, R)>
    where
        L: BlockLayout,
        R: BlockLayout,
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
    fn truncate(self, max: usize) -> (usize, Self);
}

pub trait Concatenate: LinearLayout + Sized {
    // TODO: Using morpheme iterators (which include byte indices) can be great for efficient
    // repetition and avoiding excess cloning and ridiculous allocations, but requires an upper
    // bound to avoid some bad behaviors, namely divergence (infinite looping). Note too that byte
    // indices are only useful if their associated buffer is also available (i.e., the `str`).
    // Perhaps `concatenate` should accept another `Self` and provide a default implementation in
    // terms of `extend`?
    fn concatenate<I>(self, morphemes: I) -> (usize, Self)
    where
        I: IntoIterator,
        I::Item: Morpheme;

    // TODO: The strange "associated type constructor" relationship between `Morpheme` and
    //       `Morpheme::Annex11` is a bit of a problem here. We must express that `I::Item` is the
    //       `Annex11` type of some `Morpheme` type. These types are meant to be the same, but this
    //       is not enforced by the type system and so must be expressed explicitly when used this
    //       way. Is there some way to introduce a lifetime parameter to `Morpheme` instead?
    fn extend<I>(self, min: usize, morphemes: I) -> (usize, Self)
    where
        I: IntoIterator,
        I::IntoIter: Clone,
        I::Item: Morpheme,
    {
        let mut width = self.width();
        self.concatenate(morphemes.into_iter().cycle().take_while(|morpheme| {
            width = width
                .checked_add(morpheme.width())
                .expect("overflow extending text");
            width < min
        }))
    }
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

// TODO: Since this provides a high degree of control, perhaps this trait/operation should be named
//       "merge" instead of "overlay". That of course would require renaming types that already use
//       the term "merge" though. Hmm.
pub trait Overlay: Encoded {
    type Blend<'o>;
    type Output: Encoded;

    fn overlay_with<F>(self, f: F) -> Self::Output
    where
        F: FnMut(Self::Blend<'_>) -> Receipt;
}

pub trait TryOverlay: Encoded {
    type Error;
    type Blend<'o>;
    type Output: Encoded;

    fn try_overlay_with<F>(self, f: F) -> Result<Self::Output, Self::Error>
    where
        F: FnMut(Self::Blend<'_>) -> Receipt;
}

impl<T> TryOverlay for T
where
    T: Overlay,
{
    type Error = Infallible;
    type Blend<'o> = T::Blend<'o>;
    type Output = T::Output;

    fn try_overlay_with<F>(self, f: F) -> Result<Self::Output, Self::Error>
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
