pub mod ops;

use itertools::Itertools;
use std::borrow::Cow;
use std::convert::Infallible;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::ops::Range;
use std::slice::SliceIndex;

use crate::breadth::Breadth;
use crate::slice::{SliceExt as _, SliceProjection};
use crate::{IntoWritten, MoveCow};

const CR: u8 = b'\r';
const LF: u8 = b'\n';

#[derive(Clone, Copy, Debug)]
pub enum BlockTextError {
    Control(ControlError),
    Morphology(MorphologyError),
}

impl From<ControlError> for BlockTextError {
    fn from(error: ControlError) -> Self {
        BlockTextError::Control(error)
    }
}

impl From<MorphologyError> for BlockTextError {
    fn from(error: MorphologyError) -> Self {
        BlockTextError::Morphology(error)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ControlError;

impl From<Infallible> for ControlError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MorphologyError;

impl From<Infallible> for MorphologyError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BoundaryError;

impl From<Infallible> for BoundaryError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Indexed<N, T> {
    pub index: N,
    pub text: T,
}

impl<N, T> Indexed<N, T> {
    pub fn into_text(self) -> T {
        self.text
    }

    pub fn map_index<U, F>(self, f: F) -> Indexed<U, T>
    where
        F: FnOnce(N) -> U,
    {
        let Indexed { index, text } = self;
        Indexed {
            index: f(index),
            text,
        }
    }

    pub fn map_text<U, F>(self, f: F) -> Indexed<N, U>
    where
        F: FnOnce(T) -> U,
    {
        let Indexed { index, text } = self;
        Indexed {
            index,
            text: f(text),
        }
    }

    pub fn as_ref(&self) -> Indexed<N, &T>
    where
        N: Clone,
    {
        Indexed {
            index: self.index.clone(),
            text: &self.text,
        }
    }

    pub fn as_mut(&mut self) -> Indexed<N, &mut T>
    where
        N: Clone,
    {
        Indexed {
            index: self.index.clone(),
            text: &mut self.text,
        }
    }
}

impl<N, T> Indexed<N, Option<T>> {
    pub fn transpose(self) -> Option<Indexed<N, T>> {
        let Indexed { index, text } = self;
        text.map(|text| Indexed { index, text })
    }
}

impl<N, T, E> Indexed<N, Result<T, E>> {
    pub fn transpose(self) -> Result<Indexed<N, T>, E> {
        let Indexed { index, text } = self;
        text.map(|text| Indexed { index, text })
    }
}

impl<T> Indexed<usize, T> {
    pub fn zero(text: T) -> Self {
        Indexed { index: 0, text }
    }
}

impl<N, T> From<(N, T)> for Indexed<N, T> {
    fn from((index, text): (N, T)) -> Self {
        Indexed { index, text }
    }
}

// TODO: Remove this and use `NonZeroUsize::MIN` instead.
trait NonZeroUsizeExt {
    const ONE: Self;
}

impl NonZeroUsizeExt for NonZeroUsize {
    // SAFETY: The input `usize` is never zero (it is always the literal `1`).
    const ONE: Self = unsafe { NonZeroUsize::new_unchecked(1) };
}

// Though this trait has the shape of an `AsMut` conversion, it may convert `self` prior to
// returning its reference, so it uses "to" nomenclature rather than "as".
trait ToStringMut {
    fn to_string_mut(&mut self) -> &mut String;
}

impl<'t> ToStringMut for Cow<'t, str> {
    fn to_string_mut(&mut self) -> &mut String {
        self.to_mut()
    }
}

impl ToStringMut for String {
    fn to_string_mut(&mut self) -> &mut String {
        self
    }
}

pub trait StrExt {
    fn has_ascii_line_breaks(&self) -> bool;

    // Control and layout points are CC, CF, ZL, and ZP. These general categories affect the flow
    // and layout of text and the behavior of output targets like TTYs and printers.
    fn has_control_or_layout_points(&self) -> bool;

    fn split_at_ascii_line_breaks(&self) -> impl '_ + Iterator<Item = &'_ str>;

    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<usize, Grapheme<'_>>>;

    fn width(&self) -> usize;
}

impl StrExt for str {
    fn has_ascii_line_breaks(&self) -> bool {
        // Detect any and all occurences of CR and LF. Note that both CR and LF are considered a
        // line break even when not adjacent to another line breaking control character (i.e., a
        // lone CR).
        self.as_bytes().iter().copied().any(ucs_ascii_is_cr_lf)
    }

    fn has_control_or_layout_points(&self) -> bool {
        self.chars().any(uax44_point_is_cc_cf_zl_zp)
    }

    // Splits over unpaired CR (unlike `str::lines`). Discards line breaking control characters.
    // Exlcudes LS and PS, which are in the BMP but not ASCII. While CR and LF are the only line
    // breaking control characters in ASCII, this function conceptually splits over ASCII control
    // characters with **mandatory** Unicode line break properties.
    fn split_at_ascii_line_breaks(&self) -> impl '_ + Iterator<Item = &'_ str> {
        fn checkpoint(head: &mut usize, index: usize, n: usize) -> Range<usize> {
            let range = *head..index.saturating_sub(n.saturating_sub(1));
            *head = index
                .checked_add(1)
                .expect("overflow splitting text at ASCII line breaks");
            range
        }

        // This implementation depends on CR and LF never occuring as part of a plural code point
        // sequence in UTF-8. While this is true of CR and LF, it is **not true** for all BMP and
        // Unicode line breaking code points!
        let end = self.len();
        let mut head = 0;
        self.as_bytes()
            .iter()
            .copied()
            .enumerate()
            .peekable()
            .batching(move |bytes| {
                loop {
                    return match bytes.next() {
                        // Split over CR and CR LF sequences.
                        Some((index, CR)) => Some(match bytes.peek().copied() {
                            Some((index, LF)) => {
                                bytes.next();
                                checkpoint(&mut head, index, 2)
                            }
                            _ => checkpoint(&mut head, index, 1),
                        }),
                        // Split over LF.
                        Some((index, LF)) => Some(checkpoint(&mut head, index, 1)),
                        Some(_) => continue,
                        // Yield the remainder at EoT.
                        None => (head <= end).then(|| checkpoint(&mut head, end, 0)),
                    };
                }
            })
            .map(|range| self.get(range).expect("invalid UTF-8 slice"))
    }

    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<usize, Grapheme<'_>>> {
        uax29_text_grapheme_indices(self)
    }

    fn width(&self) -> usize {
        uax11_text_width_ambiguous_non_cjk(self)
    }
}

pub trait Strip: IntoWritten + Sized {
    // Like `str::replace`, but strictly removes and does not copy when the `Pattern` matches
    // nothing.
    // TODO: Implement this in terms of `std::str::Pattern` when it is stabilized.
    fn strip<F>(self, pattern: F) -> MoveCow<Self>
    where
        F: FnMut(char) -> bool;

    fn strip_control_and_layout_points(self) -> MoveCow<Self> {
        self.strip(uax44_point_is_cc_cf_zl_zp)
    }
}

impl<T> Strip for T
where
    T: AsRef<str> + IntoWritten,
    T::Written: From<String>,
{
    // TODO: Oh man, test this.
    fn strip<F>(self, pattern: F) -> MoveCow<Self>
    where
        F: FnMut(char) -> bool,
    {
        fn checkpoint(head: &mut usize, index: usize, matched: &str) -> Range<usize> {
            let range = *head..index;
            *head = index
                .checked_add(matched.len())
                .expect("overflow stripping text");
            range
        }

        fn push(stripped: &mut String, haystack: &str, index: impl SliceIndex<str, Output = str>) {
            stripped.push_str(haystack.get(index).expect("invalid UTF-8 slice"));
        }

        let haystack = self.as_ref();
        let mut matches = haystack.match_indices(pattern);
        let mut head = 0usize;
        if let Some((index, matched)) = matches.next() {
            let mut stripped = String::with_capacity(haystack.len());
            let mut checkpoint_and_push = |index, matched| {
                push(
                    &mut stripped,
                    haystack,
                    checkpoint(&mut head, index, matched),
                );
            };
            checkpoint_and_push(index, matched);
            for (index, matched) in matches {
                checkpoint_and_push(index, matched);
            }
            push(&mut stripped, haystack, head..);
            MoveCow::Written(stripped.into())
        }
        else {
            MoveCow::Unwritten(self)
        }
    }
}

pub trait Empty {
    const EMPTY: Self;
}

impl<'t> Empty for Cow<'t, str> {
    const EMPTY: Self = Cow::Borrowed("");
}

impl<'t> Empty for &'t str {
    const EMPTY: Self = "";
}

impl<'t> Empty for &'t String {
    const EMPTY: Self = &String::new();
}

impl Empty for String {
    const EMPTY: Self = String::new();
}

// TODO: Use the term "blank" instead of "space" for morphemes. That is, abstract the notion of
//       non-display morphemes. "Space" is probably more appropriate for more general text types
//       though. See other TODOs about the terms "empty" and "blank" (remove "zero").
pub trait Blank: MorphemeFamily {
    const BLANK: MorphemeFor<'static, Self>;
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct Grapheme<'t> {
    text: Cow<'t, str>,
}

impl<'t> Grapheme<'t> {
    const fn unchecked(text: Cow<'t, str>) -> Self {
        Grapheme { text }
    }

    pub fn from_point(point: char) -> Grapheme<'static> {
        Grapheme::from(point)
    }

    pub fn into_owned(self) -> Grapheme<'static> {
        let Grapheme { text } = self;
        Grapheme {
            text: text.into_owned().into(),
        }
    }

    pub fn into_string(self) -> Cow<'t, str> {
        self.text
    }

    pub fn points(&self) -> impl '_ + Iterator<Item = char> {
        self.text.chars()
    }

    pub fn width(&self) -> usize {
        self.text.as_ref().width()
    }
}

impl<'t> AsRef<str> for Grapheme<'t> {
    fn as_ref(&self) -> &str {
        self.text.as_ref()
    }
}

// No single Unicode code point encodes more than one grapheme.
impl<'t> From<char> for Grapheme<'t> {
    fn from(point: char) -> Self {
        Grapheme::unchecked(point.to_string().into())
    }
}

impl<'t> From<Flex<'t>> for Grapheme<'t> {
    fn from(morpheme: Flex<'t>) -> Self {
        match morpheme {
            Flex::Narrow(narrow) => narrow.into(),
            Flex::Wide(wide) => wide.into(),
        }
    }
}

impl<'t, N> From<Indexed<N, Grapheme<'t>>> for Grapheme<'t> {
    fn from(indexed: Indexed<N, Grapheme<'t>>) -> Self {
        indexed.into_text()
    }
}

impl<'t> From<Narrow<'t>> for Grapheme<'t> {
    fn from(narrow: Narrow<'t>) -> Self {
        narrow.grapheme
    }
}

impl<'t> From<Wide<'t>> for Grapheme<'t> {
    fn from(wide: Wide<'t>) -> Self {
        wide.grapheme
    }
}

impl<'t> TryFrom<Cow<'t, str>> for Grapheme<'t> {
    type Error = MorphologyError;

    fn try_from(text: Cow<'t, str>) -> Result<Self, Self::Error> {
        if text.as_ref().graphemes().take(2).count() == 1 {
            Ok(Grapheme::unchecked(text))
        }
        else {
            Err(MorphologyError)
        }
    }
}

impl<'t> TryFrom<&'t str> for Grapheme<'t> {
    type Error = MorphologyError;

    fn try_from(text: &'t str) -> Result<Self, Self::Error> {
        Cow::from(text).try_into()
    }
}

impl<'t> TryFrom<String> for Grapheme<'t> {
    type Error = MorphologyError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Cow::from(text).try_into()
    }
}

// TODO: Consider renaming this to `Content`. While it does not correspond directly to the original
//       `Content` trait, it is somewhat similar and is implemented by textual types composed
//       entirely of morphemes. This is the only text supported by the crate and so is essentially
//       implemented by "content" types.
pub trait BlockText: BlockTextProjection<Text = <Self as BlockText>::Text, Output = Self> {
    // TODO: Should this have a bound on `RawText`?
    type Text;
    // TODO: Don't use family/kind here. Instead, only use family as the input type parameter for
    //       types like `Segment` (probably only `Segment`) and then associate the parameterized
    //       type when implementing `BlockText`.
    //type MorphemeFamily: MorphemeFamily;
    type Morpheme<'t>: Morpheme<'t>
    where
        Self: 't;
    type Index: Eq;

    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<Self::Index, Grapheme<'_>>>;

    // TODO: `Encoded` types must never allow construction from text containing non-morphemes.
    //       Ideally then, this function need not examine the text and can instead construct
    //       morphemes via `unchecked`! In a default implementation though, shenanigans are
    //       possible, especially if downstream code implements this trait (which would implicitly
    //       grant access to `unchecked`, which is intentionally not in the public API). This trait
    //       could be sealed or an internal function could be used to trivially implement this
    //       function instead of providing a default.
    fn morphemes(
        &self,
    ) -> impl '_ + Clone + Iterator<Item = Indexed<Self::Index, Self::Morpheme<'_>>> {
        self.graphemes()
            .map(|indexed| indexed.map_text(Self::Morpheme::try_from).transpose())
            .map(move |morpheme| match morpheme {
                Ok(morpheme) => morpheme,
                _ => panic!("non-morpheme in text"),
            })
    }
}

pub trait BlockTextProjection {
    type Text;
    type Output: BlockText<Text = Self::Text>;
    type Mapped<T>: BlockTextProjection
    where
        T: BlockText;

    fn into_block_text(self) -> Self::Output;

    fn map_block_text<T, F>(self, f: F) -> Self::Mapped<T>
    where
        T: BlockText,
        F: FnOnce(Self::Output) -> T;

    fn as_block_text(&self) -> &Self::Output;

    fn as_block_text_mut(&mut self) -> &mut Self::Output;
}

impl<T> BlockTextProjection for T
where
    T: BlockText,
{
    type Text = <T as BlockText>::Text;
    type Output = T;
    type Mapped<U> = U
    where
        U: BlockText;

    fn into_block_text(self) -> Self::Output {
        self
    }

    // TODO: At time of writing, `rustc` claims that `U` and `Self::Mapped<U>` are incompatible
    //       (not the same type), despite the definition `type Mapped<U> = U;`. Because these types
    //       are always the same in this implementation, the output `U` of `F` is transmuted into
    //       `Self::Mapped<U>`.
    fn map_block_text<U, F>(self, f: F) -> Self::Mapped<U>
    where
        U: BlockText,
        F: FnOnce(Self::Output) -> U,
    {
        use std::mem;

        // SAFETY: The types `U` and `Self::Mapped<U>` must be the same or this transmutation is
        //         very likely UB and this API is unsound.
        unsafe { mem::transmute_copy::<U, Self::Mapped<U>>(&f(self)) }
    }

    fn as_block_text(&self) -> &Self::Output {
        self
    }

    fn as_block_text_mut(&mut self) -> &mut Self::Output {
        self
    }
}

// Count of columns and rows of some text. This is somewhat general, but generally considers line
// breaks.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BoundingBox<H> {
    pub width: usize,
    pub height: H,
}

// NOTE: This is not mutually exclusive with `BlockLayout`. For example, `Line` is designed to be
//       composed into a `Block` and implements `BlockLayout`, but it only ever consists of a
//       single line of text. This makes it compatible with linear operations.
//
//       This is only true where inputs and outputs are the same; a morphism **may be** okay, but a
//       transform is not. The layout traits both work this way: they only apply to closed
//       operations. This should be in the trait documentation.
pub trait LinearLayout: BlockText {
    fn width(&self) -> usize;
}

pub trait BlockLayout: BlockText {
    type Height: Copy + Eq + Into<usize> + Ord;

    // NOTE: It is important to keep these kinds of bounds distinct from `Unicode::width`,
    //       `str::len`, etc.! These functions should **always** consider the complete text as a
    //       sum. Here, ASCII line breaks are used to consider the structure of the text, and so
    //       the width bound is a maximum by line. Imagine if `str::len` or some other `len`
    //       function did this: it would be quite confusing. Do not conflate these concepts in
    //       APIs.
    fn ascii_line_break_bounds(&self) -> BoundingBox<Self::Height>;
}

// TODO: Consolidate traits like `Empty` into `RawText`. These traits are only implemented for raw
//       text types.
// TODO: This is probably the one trait that should express as many useful bounds on parent traits
//       as possible.
pub trait RawText: AsRef<str> + Empty + IntoWritten + Strip {}

impl<'t> RawText for Cow<'t, str> {}

impl<'t> RawText for &'t str {}

impl<'t> RawText for &'t String {}

impl RawText for String {}

// TODO: Consider the term "kind" instead of "family". I prefer the term "kind" in a different
//       pattern, but it works well here and has the benefit of terseness.
pub trait MorphemeFamily {
    type Morpheme<'t>: Morpheme<'t>;
}

#[derive(Debug)]
pub enum FlexFamily {}

impl MorphemeFamily for FlexFamily {
    type Morpheme<'t> = Flex<'t>;
}

#[derive(Debug)]
pub enum NarrowFamily {}

impl MorphemeFamily for NarrowFamily {
    type Morpheme<'t> = Narrow<'t>;
}

#[derive(Debug)]
pub enum WideFamily {}

impl MorphemeFamily for WideFamily {
    type Morpheme<'t> = Wide<'t>;
}

pub trait Morpheme<'t>: AsRef<str> + Into<Flex<'t>> + TryFrom<Grapheme<'t>> {
    type Family: MorphemeFamily<Morpheme<'t> = Self>;

    fn into_string(self) -> Cow<'t, str>;

    fn width(&self) -> NonZeroUsize;
}

pub type MorphemeFor<'t, M> = <M as MorphemeFamily>::Morpheme<'t>;

pub type Flex<'t> = Breadth<Narrow<'t>, Wide<'t>>;

impl<'t> Flex<'t> {
    pub fn into_owned(self) -> Flex<'t> {
        match self {
            Flex::Narrow(narrow) => Flex::Narrow(narrow.into_owned()),
            Flex::Wide(wide) => Flex::Wide(wide.into_owned()),
        }
    }

    pub fn as_grapheme(&self) -> &Grapheme<'t> {
        match self {
            Flex::Narrow(ref narrow) => narrow.as_grapheme(),
            Flex::Wide(ref wide) => wide.as_grapheme(),
        }
    }
}

impl AsRef<str> for Flex<'_> {
    fn as_ref(&self) -> &str {
        match self {
            Flex::Narrow(ref narrow) => narrow.as_ref(),
            Flex::Wide(ref wide) => wide.as_ref(),
        }
    }
}

impl<'t> From<Narrow<'t>> for Flex<'t> {
    fn from(narrow: Narrow<'t>) -> Self {
        Flex::Narrow(narrow)
    }
}

impl<'t> From<Wide<'t>> for Flex<'t> {
    fn from(wide: Wide<'t>) -> Self {
        Flex::Wide(wide)
    }
}

impl<'t> Morpheme<'t> for Flex<'t> {
    type Family = FlexFamily;

    fn into_string(self) -> Cow<'t, str> {
        match self {
            Flex::Narrow(narrow) => narrow.into_string(),
            Flex::Wide(wide) => wide.into_string(),
        }
    }

    fn width(&self) -> NonZeroUsize {
        match self {
            Flex::Narrow(_) => Narrow::WIDTH,
            Flex::Wide(_) => Wide::WIDTH,
        }
    }
}

impl<'t> TryFrom<Grapheme<'t>> for Flex<'t> {
    type Error = MorphologyError;

    fn try_from(grapheme: Grapheme<'t>) -> Result<Self, Self::Error> {
        match NonZeroUsize::new(grapheme.width()) {
            Some(Narrow::WIDTH) => Ok(Narrow::unchecked(grapheme).into()),
            Some(Wide::WIDTH) => Ok(Wide::unchecked(grapheme).into()),
            _ => Err(MorphologyError),
        }
    }
}

// Narrow **or ambiguous one-column width**.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct Narrow<'t> {
    grapheme: Grapheme<'t>,
}

impl<'t> Narrow<'t> {
    pub const WIDTH: NonZeroUsize = NonZeroUsize::ONE;

    const fn unchecked(grapheme: Grapheme<'t>) -> Self {
        Narrow { grapheme }
    }

    pub const fn space() -> Narrow<'static> {
        Narrow::unchecked(Grapheme::unchecked(Cow::Borrowed(" ")))
    }

    pub fn into_owned(self) -> Narrow<'static> {
        let Narrow { grapheme } = self;
        Narrow {
            grapheme: grapheme.into_owned(),
        }
    }

    pub fn as_grapheme(&self) -> &Grapheme<'t> {
        &self.grapheme
    }

    pub fn is_ambiguous(&self) -> bool {
        // This function is defined here to avoid its use elsewhere: text width is non-CJK when
        // ambiguous.
        fn uax11_text_width_ambiguous_cjk(text: &str) -> usize {
            use unicode_width::UnicodeWidthStr;

            UnicodeWidthStr::width_cjk(text)
        }

        let text = self.as_ref();
        uax11_text_width_ambiguous_cjk(text) != uax11_text_width_ambiguous_non_cjk(text)
    }
}

impl<'t> AsRef<str> for Narrow<'t> {
    fn as_ref(&self) -> &str {
        self.grapheme.as_ref()
    }
}

impl<'t> Morpheme<'t> for Narrow<'t> {
    type Family = NarrowFamily;

    fn into_string(self) -> Cow<'t, str> {
        self.grapheme.into_string()
    }

    fn width(&self) -> NonZeroUsize {
        Self::WIDTH
    }
}

impl<'t> TryFrom<Grapheme<'t>> for Narrow<'t> {
    type Error = MorphologyError;

    fn try_from(grapheme: Grapheme<'t>) -> Result<Self, Self::Error> {
        if grapheme.width() == Narrow::WIDTH.get() {
            Ok(Narrow::unchecked(grapheme))
        }
        else {
            Err(MorphologyError)
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct Wide<'t> {
    grapheme: Grapheme<'t>,
}

impl<'t> Wide<'t> {
    // SAFETY: The input `usize` is never zero (it is always the literal `2`).
    pub const WIDTH: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(2) };

    const fn unchecked(grapheme: Grapheme<'t>) -> Self {
        Wide { grapheme }
    }

    pub const fn space() -> Wide<'static> {
        Wide::unchecked(Grapheme::unchecked(Cow::Borrowed("　")))
    }

    pub fn into_owned(self) -> Wide<'static> {
        let Wide { grapheme } = self;
        Wide {
            grapheme: grapheme.into_owned(),
        }
    }

    pub fn as_grapheme(&self) -> &Grapheme<'t> {
        &self.grapheme
    }
}

impl<'t> AsRef<str> for Wide<'t> {
    fn as_ref(&self) -> &str {
        self.grapheme.as_ref()
    }
}

impl<'t> Morpheme<'t> for Wide<'t> {
    type Family = WideFamily;

    fn into_string(self) -> Cow<'t, str> {
        self.grapheme.into_string()
    }

    fn width(&self) -> NonZeroUsize {
        Self::WIDTH
    }
}

impl<'t> TryFrom<Grapheme<'t>> for Wide<'t> {
    type Error = MorphologyError;

    fn try_from(grapheme: Grapheme<'t>) -> Result<Self, Self::Error> {
        if grapheme.width() == Wide::WIDTH.get() {
            Ok(Wide::unchecked(grapheme))
        }
        else {
            Err(MorphologyError)
        }
    }
}

// This makes interpreting linear text types as blocks explicit.
#[derive(Debug)]
#[repr(transparent)]
struct AsBlockLayout<'t, T>(&'t T);

impl<'t, T> BlockLayout for AsBlockLayout<'t, T>
where
    T: LinearLayout,
{
    type Height = NonZeroUsize;

    fn ascii_line_break_bounds(&self) -> BoundingBox<Self::Height> {
        BoundingBox {
            width: self.0.width(),
            height: NonZeroUsize::ONE,
        }
    }
}

impl<'t, T> BlockText for AsBlockLayout<'t, T>
where
    T: BlockText,
{
    type Text = <T as BlockText>::Text;
    type Morpheme<'m> = T::Morpheme<'m>
    where
        Self: 'm;
    type Index = T::Index;

    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<Self::Index, Grapheme<'_>>> {
        self.0.graphemes()
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Annotated<T, A = ()> {
    text: T,
    annotation: A,
}

impl<T, A> Annotated<T, A> {
    pub const fn annotate(text: T, annotation: A) -> Self {
        Annotated { text, annotation }
    }

    pub fn from_text(text: T) -> Self
    where
        A: Default,
    {
        Annotated {
            text,
            annotation: Default::default(),
        }
    }

    pub fn map_text<U, F>(self, f: F) -> Annotated<U, A>
    where
        F: FnOnce(T) -> U,
    {
        let Annotated { text, annotation } = self;
        Annotated {
            text: f(text),
            annotation,
        }
    }

    pub fn map_annotation<U, F>(self, f: F) -> Annotated<T, U>
    where
        F: FnOnce(A) -> U,
    {
        let Annotated { text, annotation } = self;
        Annotated {
            text,
            annotation: f(annotation),
        }
    }
}

impl<T, A> BlockTextProjection for Annotated<T, A>
where
    T: BlockText,
{
    type Text = <T as BlockText>::Text;
    type Output = T;
    type Mapped<U> = Annotated<U, A>
    where
        U: BlockText;

    fn into_block_text(self) -> Self::Output {
        self.text
    }

    fn map_block_text<U, F>(self, f: F) -> Self::Mapped<U>
    where
        U: BlockText,
        F: FnOnce(Self::Output) -> U,
    {
        self.map_text(f)
    }

    fn as_block_text(&self) -> &Self::Output {
        &self.text
    }

    fn as_block_text_mut(&mut self) -> &mut Self::Output {
        &mut self.text
    }
}

pub type SegmentFor<T, M> = Segment<<T as BlockTextProjection>::Text, M>;

// TODO: Segments ignore non-ASCII line breaks (by design). Make sure this is documented.
// TODO: Consider `unicode-linebreak` or something similar if it seems that support for line
//       breaking Unicode control characters like LS and PS is justified.
// TODO: The derived implementations do not depend on the type parameter `M`. Implement them
//       explicitly to reflect this.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Segment<T, M = FlexFamily> {
    text: T,
    _phantom: PhantomData<fn() -> M>,
}

impl<T, M> Segment<T, M>
where
    T: RawText,
    M: MorphemeFamily,
{
    fn unchecked(text: T) -> Self {
        Segment {
            text,
            _phantom: PhantomData,
        }
    }

    pub fn try_from_string<U>(text: U) -> Result<Self, MorphologyError>
    where
        T: TryFrom<MoveCow<U>>,
        U: RawText,
    {
        let text: T = text
            .strip_control_and_layout_points()
            .try_into()
            .map_err(|_| MorphologyError)?;
        if text
            .as_ref()
            .graphemes()
            .map(Indexed::into_text)
            .map(MorphemeFor::<M>::try_from)
            .all(|morpheme| morpheme.is_ok())
        {
            Ok(Segment::unchecked(text))
        }
        else {
            Err(MorphologyError)
        }
    }

    pub fn try_from_string_or_joined<U>(text: U) -> Result<Self, MorphologyError>
    where
        T: TryFrom<MoveCow<String>> + TryFrom<MoveCow<U>>,
        U: RawText,
    {
        let mut lines = text.as_ref().split_at_ascii_line_breaks().peekable();
        let first = lines.next();
        if lines.peek().is_some() {
            Segment::try_from_string(first.into_iter().chain(lines).join(""))
        }
        else {
            drop(lines);
            Segment::try_from_string(text)
        }
    }

    pub fn from_string_or_empty<U>(text: U) -> Self
    where
        T: TryFrom<MoveCow<U>>,
        U: RawText,
    {
        match Segment::try_from_string(text) {
            Ok(text) => text,
            _ => Segment::empty(),
        }
    }

    pub fn assert<U>(text: U) -> Self
    where
        T: TryFrom<MoveCow<U>>,
        U: RawText,
    {
        Segment::try_from_string(text).expect("failed to construct block text")
    }

    pub const fn empty() -> Self {
        Segment {
            text: T::EMPTY,
            _phantom: PhantomData,
        }
    }

    pub fn try_map_string<U, F>(self, f: F) -> Result<Segment<U, M>, MorphologyError>
    where
        U: RawText + TryFrom<MoveCow<U>>,
        F: FnOnce(T) -> U,
    {
        let Segment { text, .. } = self;
        Segment::try_from_string(f(text))
    }

    pub fn as_str(&self) -> &str {
        AsRef::<str>::as_ref(self)
    }

    // TODO: These bounds may be more specific than necessary: only the block bounds are needed
    //       here!
    // CLIPPY: This appears to be a false positive. An explicit lifetime is necessary for the GATs.
    #[allow(clippy::needless_lifetimes)]
    pub fn as_block_layout<'b>(
        &'b self,
    ) -> impl 'b + BlockLayout<Morpheme<'b> = MorphemeFor<'b, M>, Index = usize> {
        AsBlockLayout(self)
    }
}

impl<'t, M> Segment<Cow<'t, str>, M>
where
    M: MorphemeFamily,
{
    pub fn into_owned(self) -> Segment<Cow<'static, str>, M> {
        let Segment { text, .. } = self;
        Segment::unchecked(text.into_owned().into())
    }
}

impl<T, M> ops::Append for Segment<T, M>
where
    T: RawText + ToStringMut,
    M: MorphemeFamily,
{
    fn append(mut self, rhs: Self) -> Self {
        self.text.to_string_mut().push_str(rhs.as_str());
        self
    }
}

impl<T, M> AsRef<str> for Segment<T, M>
where
    T: AsRef<str>,
{
    fn as_ref(&self) -> &str {
        self.text.as_ref()
    }
}

impl<T, M> BlockText for Segment<T, M>
where
    T: RawText,
    M: MorphemeFamily,
{
    type Text = T;
    type Morpheme<'t> = MorphemeFor<'t, M>
    where
        Self: 't;
    type Index = usize;

    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<Self::Index, Grapheme<'_>>> {
        self.text.as_ref().graphemes()
    }
}

impl<T, M> ops::Extend for Segment<T, M>
where
    T: RawText + ToStringMut,
    M: MorphemeFamily,
{
    fn extend<'t, I>(&'t mut self, morphemes: I) -> usize
    where
        I: IntoIterator<Item = Self::Morpheme<'t>>,
    {
        self.text
            .to_string_mut()
            .extend(morphemes.into_iter().map(Morpheme::into_string));
        self.width()
    }
}

impl<T, M> LinearLayout for Segment<T, M>
where
    T: RawText,
    M: MorphemeFamily,
{
    fn width(&self) -> usize {
        self.text.as_ref().width()
    }
}

impl<T, M> ops::Truncate for Segment<T, M>
where
    T: RawText + ToStringMut,
    M: MorphemeFamily,
{
    fn truncate(&mut self, max: usize) -> usize {
        let mut width = 0usize;
        // This iterator expression cannot use `find`, because it borrows the iterator, which
        // prevents the mutable borrow in the branch.
        if let Some(len) = self
            .text
            .as_ref()
            .graphemes()
            .skip_while(|grapheme| {
                width = width
                    .checked_add(grapheme.text.width().into())
                    .expect("overflow truncating text");
                width <= max
            })
            .take(1)
            .last()
            .map(|grapheme| grapheme.index)
        {
            self.text.to_string_mut().truncate(len);
        }
        self.width()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LineIndex {
    pub segment: usize,
    pub byte: usize,
}

// NOTE: Annotation type parameters (`A`) are captured by `T` here. That is, `Annotated` is
//       abstracted such that `Line` has fewer type parameters and need not forward nor manage `A`.
//       This will probably make it easier to support `Fill` types with "computed text".
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Line<T> {
    // TODO: Perhaps segments ought to be stored in a `VecDeque` instead? If prepending becomes
    //       necessary in code written against `Line`, consider making this change.
    segments: Vec<T>,
}

impl<T> Line<T> {
    pub const fn empty() -> Self {
        Line {
            segments: Vec::new(),
        }
    }
}

impl<T, M> Line<T>
where
    T: BlockTextProjection<Output = Segment<<T as BlockTextProjection>::Text, M>>,
    T::Text: RawText,
    M: MorphemeFamily,
{
    pub fn push(&mut self, segment: impl Into<T>) {
        self.segments.push(segment.into());
    }

    pub fn segments<'s>(&'s self) -> impl 's + SliceProjection<Item = SegmentFor<T, M>>
    where
        M: 's,
    {
        self.segments
            .as_slice()
            .project(BlockTextProjection::as_block_text)
    }

    pub fn to_string<'s>(&'s self) -> Cow<'s, str>
    where
        M: 's,
    {
        let segments = self.segments();
        match segments.len() {
            0 => "".into(),
            // TODO: Why can this not be done through the slice projection...? Fix this, if
            //       possible.
            //1 => segments.get(0).unwrap().as_str().into(),
            1 => self.segments[0].as_block_text().as_ref().into(),
            _ => segments.iter().map(Segment::as_str).join("").into(),
        }
    }

    // TODO: These bounds may be more specific than necessary: only the block bounds are needed
    //       here!
    // CLIPPY: This appears to be a false positive. An explicit lifetime is necessary for the GATs.
    #[allow(clippy::needless_lifetimes)]
    pub fn as_block_layout<'b>(
        &'b self,
    ) -> impl 'b + BlockLayout<Morpheme<'b> = MorphemeFor<'b, M>, Index = LineIndex> {
        AsBlockLayout(self)
    }
}

impl<'t, T, M> Line<T>
where
    T: BlockTextProjection<
        Output = Segment<<T as BlockTextProjection>::Text, M>,
        Text = Cow<'t, str>,
    >,
    M: MorphemeFamily,
{
    pub fn into_owned(self) -> Line<T::Mapped<Segment<Cow<'static, str>, M>>> {
        let Line { segments } = self;
        Line {
            segments: segments
                .into_iter()
                .map(|segment| segment.map_block_text(Segment::into_owned))
                .collect(),
        }
    }
}

impl<T> Default for Line<T> {
    fn default() -> Self {
        Line {
            segments: Default::default(),
        }
    }
}

impl<T, M> BlockText for Line<T>
where
    T: BlockTextProjection<Output = Segment<<T as BlockTextProjection>::Text, M>>,
    T::Text: RawText,
    M: MorphemeFamily,
{
    type Text = T::Text;
    type Morpheme<'t> = MorphemeFor<'t, M>
    where
        Self: 't;
    type Index = LineIndex;

    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<Self::Index, Grapheme<'_>>> {
        self.segments
            .iter()
            .map(BlockTextProjection::as_block_text)
            .enumerate()
            .flat_map(|(index, segment)| {
                segment.graphemes().map(move |grapheme| {
                    grapheme.map_index(|byte| LineIndex {
                        segment: index,
                        byte,
                    })
                })
            })
    }
}

impl<T> From<Vec<T>> for Line<T> {
    fn from(segments: Vec<T>) -> Self {
        Line { segments }
    }
}

impl<T> FromIterator<T> for Line<T> {
    fn from_iter<I>(segments: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Line::from(segments.into_iter().collect::<Vec<_>>())
    }
}

impl<T, M> LinearLayout for Line<T>
where
    T: BlockTextProjection<Output = Segment<<T as BlockTextProjection>::Text, M>>,
    T::Text: RawText,
    M: MorphemeFamily,
{
    fn width(&self) -> usize {
        self.segments
            .iter()
            .map(BlockTextProjection::as_block_text)
            .map(LinearLayout::width)
            .sum()
    }
}

fn ucs_ascii_is_cr_lf(byte: u8) -> bool {
    matches!(byte, CR | LF)
}

fn uax44_point_is_cc_cf_zl_zp(point: char) -> bool {
    use unicode_properties::{GeneralCategory, UnicodeGeneralCategory};

    use GeneralCategory::{Control, Format};

    matches!(point.general_category(), Control | Format)
}

fn uax11_point_width_ambiguous_non_cjk(point: char) -> usize {
    use unicode_width::UnicodeWidthChar;

    UnicodeWidthChar::width(point).unwrap_or(0)
}

// Here, "ambiguous non-CJK" means that UAX11 ambiguous graphemes are assigned the "non-CJK" column
// width of one (rather than two, which is typically more compatible in CJK contexts). Generally,
// ambiguous and halfwidth graphemes are both mapped to narrow morphemes and are treated the same.
fn uax11_text_width_ambiguous_non_cjk(text: &str) -> usize {
    use unicode_width::UnicodeWidthStr;

    // NOTE: This considers some potentially troublesome ASCII whitespace characters as zero-width,
    //       which works well here! For example, TAB is zero-width and so is not a morpheme.
    UnicodeWidthStr::width(text)
}

fn uax29_text_graphemes(text: &str) -> impl '_ + Clone + Iterator<Item = Grapheme<'_>> {
    use unicode_segmentation::UnicodeSegmentation;

    UnicodeSegmentation::graphemes(text, true)
        .map(Cow::from)
        .map(Grapheme::unchecked)
}

fn uax29_text_grapheme_indices(
    text: &str,
) -> impl '_ + Clone + Iterator<Item = Indexed<usize, Grapheme<'_>>> {
    use unicode_segmentation::UnicodeSegmentation;

    UnicodeSegmentation::grapheme_indices(text, true)
        .map(Indexed::from)
        .map(|grapheme| grapheme.map_text(Cow::from).map_text(Grapheme::unchecked))
}

#[cfg(test)]
mod tests {
    use crate::text::StrExt as _;

    #[test]
    fn split_at_ascii_line_breaks() {
        fn lines(text: &str) -> Vec<&str> {
            text.split_at_ascii_line_breaks().collect()
        }

        assert_eq!(lines(""), vec![""]);
        assert_eq!(lines("\n"), vec!["", ""]);
        assert_eq!(lines("a\nb"), vec!["a", "b"]);
        assert_eq!(lines("a\r\nb"), vec!["a", "b"]);
        assert_eq!(lines("a\r\r\nb"), vec!["a", "", "b"]);
        assert_eq!(lines("a\n\rb"), vec!["a", "", "b"]);
        assert_eq!(lines("a\r\r\n\r\nb"), vec!["a", "", "", "b"]);
        assert_eq!(lines("\na"), vec!["", "a"]);
        assert_eq!(lines("\n\na"), vec!["", "", "a"]);
        assert_eq!(lines("a\n"), vec!["a", ""]);
        assert_eq!(lines("a\n\n"), vec!["a", "", ""]);
    }
}
