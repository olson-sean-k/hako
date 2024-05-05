pub mod ops;

use itertools::Itertools;
use std::borrow::Cow;
use std::convert::Infallible;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::ops::Range;

use crate::breadth::Breadth;
use crate::slice::SliceProjection;

// TODO: Use "uax" isntead of "annex".

const CR: u8 = b'\r';
const LF: u8 = b'\n';

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

// The index type parameter `N` is in a somewhat unconventional order, but appears last so that a
// default is possible.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Indexed<T, N = usize> {
    pub text: T,
    pub index: N,
}

impl<T, N> Indexed<T, N> {
    pub fn into_text(self) -> T {
        self.text
    }

    pub fn map<U, F>(self, f: F) -> Indexed<U, N>
    where
        F: FnOnce(T) -> U,
    {
        let Indexed { index, text } = self;
        Indexed {
            index,
            text: f(text),
        }
    }

    pub fn as_ref(&self) -> Indexed<&T, N>
    where
        N: Clone,
    {
        Indexed {
            index: self.index.clone(),
            text: &self.text,
        }
    }

    pub fn as_mut(&mut self) -> Indexed<&mut T, N>
    where
        N: Clone,
    {
        Indexed {
            index: self.index.clone(),
            text: &mut self.text,
        }
    }
}

impl<T, N> Indexed<Option<T>, N> {
    pub fn transpose(self) -> Option<Indexed<T, N>> {
        let Indexed { index, text } = self;
        text.map(|text| Indexed { index, text })
    }
}

impl<T, E, N> Indexed<Result<T, E>, N> {
    pub fn transpose(self) -> Result<Indexed<T, N>, E> {
        let Indexed { index, text } = self;
        text.map(|text| Indexed { index, text })
    }
}

impl<T> Indexed<T, usize> {
    pub fn zero(text: T) -> Self {
        Indexed { index: 0, text }
    }
}

impl<T, N> From<(N, T)> for Indexed<T, N> {
    fn from((index, text): (N, T)) -> Self {
        Indexed { index, text }
    }
}

trait NonZeroUsizeExt {
    const ONE: Self;
}

impl NonZeroUsizeExt for NonZeroUsize {
    // SAFETY: The input `usize` is never zero (it is always the literal `1`).
    const ONE: Self = unsafe { NonZeroUsize::new_unchecked(1) };
}

trait StrExt {
    fn has_ascii_line_breaks(&self) -> bool;

    // TODO: Perhaps cloning can be avoided or deferred by implementing a similar consuming
    //       function for types like `Cow<str>`.
    fn split_at_ascii_line_breaks(&self) -> impl '_ + Iterator<Item = &'_ str>;
}

impl StrExt for str {
    fn has_ascii_line_breaks(&self) -> bool {
        // Detect any and all occurences of CR and LF. Note that both CR and LF are considered a
        // line break even when not adjacent to another control line breaking control character
        // (i.e., a lone CR).
        self.as_bytes().iter().copied().any(is_ascii_line_break)
    }

    // Splits over unpaired CR (unlike `str::lines`). Discards line breaking control characters.
    // Exlcudes LS and PS, which are in the BMP but not ASCII. While CR and LF are the only line
    // breaking control characters in ASCII, this function conceptually splits over ASCII control
    // characters with **mandatory** Unicode line break properties.
    fn split_at_ascii_line_breaks(&self) -> impl '_ + Iterator<Item = &'_ str> {
        fn checkpoint(head: &mut usize, index: usize, n: usize) -> Range<usize> {
            let range = *head..index.saturating_sub(n.saturating_sub(1));
            *head = index.checked_add(1).expect("overflow in index");
            range
        }

        // This implementation depends on CR and LF never occuring as part of a plural code point
        // sequence in UTF-8. This is not true of all BMP and Unicode line breaking code points!
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
}

trait ToStringMut: Unicode {
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

pub trait Empty {
    const EMPTY: Self;
}

impl<'t> Empty for Cow<'t, str> {
    const EMPTY: Self = Cow::Borrowed("");
}

impl<'t> Empty for &'t str {
    const EMPTY: Self = "";
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

// TODO: Is there a better name for this? This is implemented a bit too broadly to function as a
//       typical extension trait (i.e., `StrExt`), but `str` and friends are already Unicode.
//       Moreoever, this trait adopts specific answers to somewhat ambiguous questions in Unicode,
//       especially `width`.
pub trait Unicode {
    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<Grapheme<'_>>>;

    fn width(&self) -> usize;
}

impl<T> Unicode for T
where
    T: ?Sized + SliceProjection,
    T::Item: Unicode,
{
    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<Grapheme<'_>>> {
        self.iter().flat_map(|unicode| unicode.graphemes())
    }

    fn width(&self) -> usize {
        self.iter().map(|unicode| unicode.width()).sum()
    }
}

impl Unicode for char {
    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<Grapheme<'_>>> {
        Some(Indexed::zero(Grapheme::from(*self))).into_iter()
    }

    fn width(&self) -> usize {
        annex11_point_width_ambiguous_non_cjk(*self)
    }
}

impl<'t> Unicode for Cow<'t, str> {
    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<Grapheme<'_>>> {
        self.as_ref().graphemes()
    }

    fn width(&self) -> usize {
        self.as_ref().width()
    }
}

impl Unicode for str {
    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<Grapheme<'_>>> {
        annex29_text_grapheme_segmentation(self)
    }

    fn width(&self) -> usize {
        annex11_text_width_ambiguous_non_cjk(self)
    }
}

impl<'t> Unicode for &'t str {
    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<Grapheme<'_>>> {
        Unicode::graphemes(*self)
    }

    fn width(&self) -> usize {
        Unicode::width(*self)
    }
}

impl Unicode for String {
    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<Grapheme<'_>>> {
        self.as_str().graphemes()
    }

    fn width(&self) -> usize {
        self.as_str().width()
    }
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

    pub fn width(&self) -> NonZeroUsize {
        NonZeroUsize::new(Unicode::width(self)).expect("zero-width grapheme")
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

impl<'t> Unicode for Grapheme<'t> {
    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<Grapheme<'_>>> {
        self.as_ref().graphemes()
    }

    fn width(&self) -> usize {
        self.as_ref().width()
    }
}

pub trait Encoded: Unicode {
    type MorphemeFamily: MorphemeFamily;

    // TODO: `Encoded` types must never allow construction from text containing non-morphemes.
    //       Ideally then, this function need not examine the text and can instead construct
    //       morphemes via `unchecked`! In a default implementation though, shenanigans are
    //       possible, especially if downstream code implements this trait (which would implicitly
    //       grant access to `unchecked`, which is intentionally not in the public API). This trait
    //       could be sealed or an internal function could be used to trivially implement this
    //       function instead of providing a default.
    fn morphemes(
        &self,
    ) -> impl '_ + Clone + Iterator<Item = Indexed<MorphemeFor<'_, Self::MorphemeFamily>>> {
        self.graphemes()
            .map(|grapheme| {
                grapheme
                    .map(MorphemeFor::<Self::MorphemeFamily>::try_from)
                    .transpose()
            })
            .map(move |morpheme| match morpheme {
                Ok(morpheme) => morpheme,
                _ => panic!("non-morpheme in text"),
            })
    }
}

impl<T> Encoded for T
where
    T: ?Sized + SliceProjection,
    T::Item: Encoded,
{
    type MorphemeFamily = <T::Item as Encoded>::MorphemeFamily;
}

// Count of columns and rows of some text. This is somewhat general, but generally considers line
// breaks.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BoundingBox {
    pub width: usize,
    pub height: NonZeroUsize,
}

// NOTE: This is not mutually exclusive with `BlockLayout`. For example, `Line` is designed to be
//       composed into a `Block` and implements `BlockLayout`, but it only ever consists of a
//       single line of text. This makes it compatible with linear operations.
//
//       This is only true where inputs and outputs are the same; a morphism **may be** okay, but a
//       transform is not. The layout traits both work this way: they only apply to closed
//       operations. This should be in the trait documentation.
pub trait LinearLayout: Encoded {}

pub trait BlockLayout: Encoded {
    // NOTE: It is important to keep these kinds of bounds distinct from `Unicode::width`,
    //       `str::len`, etc.! These functions should **always** consider the complete text as a
    //       sum. Here, ASCII line breaks are used to consider the structure of the text, and so
    //       the width bound is a maximum by line. Imagine if `str::len` or some other `len`
    //       function did this: it would be quite confusing. Do not conflate these concepts in
    //       APIs.
    fn ascii_line_break_bounds(&self) -> BoundingBox;
}

//pub trait Morpheme: Sized {
//    type Annex11<'t>: AsRef<str> + Into<Flex<'t>> + TryFrom<Grapheme<'t>>;
//}

//pub type Annex11<'t, M> = <M as Morpheme>::Annex11<'t>;

pub trait Unchecked {}

impl<'t> Unchecked for Cow<'t, str> {}

impl<'t> Unchecked for &'t str {}

impl Unchecked for String {}

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
    type Family: MorphemeFamily;

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
        match NonZeroUsize::new(Unicode::width(&grapheme)) {
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
        todo!()
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
        if Unicode::width(&grapheme) == Narrow::WIDTH.into() {
            Ok(Narrow::unchecked(grapheme))
        }
        else {
            Err(MorphologyError)
        }
    }
}

impl<'t> Unicode for Narrow<'t> {
    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<Grapheme<'_>>> {
        self.grapheme.graphemes()
    }

    #[inline(always)]
    fn width(&self) -> usize {
        Narrow::WIDTH.into()
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
        if Unicode::width(&grapheme) == Wide::WIDTH.into() {
            Ok(Wide::unchecked(grapheme))
        }
        else {
            Err(MorphologyError)
        }
    }
}

impl<'t> Unicode for Wide<'t> {
    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<Grapheme<'_>>> {
        self.grapheme.graphemes()
    }

    #[inline(always)]
    fn width(&self) -> usize {
        Wide::WIDTH.into()
    }
}

// TODO: The types below are most analogous to `Content` types in the original design. As such,
//       maybe they ought to be parameterized by string representation rather than choosing
//       `Cow<str>`. It may be better for the API to support `Text<String, FlexFamily>`, for
//       example, so that users need not juggle lifetimes if they don't care about potential
//       performance penalties or no such penalties really apply to their use case.

// TODO: The derived implementations do not depend on the type parameter `M`. Implement them
//       explicitly to reflect this.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct Text<T, M = FlexFamily> {
    text: T,
    _phantom: PhantomData<fn() -> M>,
}

impl<T, M> Text<T, M>
where
    T: AsRef<str> + Unchecked + Unicode,
    M: MorphemeFamily,
{
    fn unchecked(text: T) -> Self {
        Text {
            text,
            _phantom: PhantomData,
        }
    }

    pub fn try_from_string(text: impl Into<T>) -> Result<Self, MorphologyError> {
        let text = text.into();
        if text
            .as_ref()
            .graphemes()
            .map(Indexed::into_text)
            .map(MorphemeFor::<M>::try_from)
            .all(|morpheme| morpheme.is_ok())
        {
            Ok(Text::unchecked(text))
        }
        else {
            Err(MorphologyError)
        }
    }

    pub fn from_string_or_empty(text: impl Into<T>) -> Self
    where
        T: Empty,
    {
        match Text::try_from_string(text) {
            Ok(text) => text,
            _ => Text::empty(),
        }
    }

    pub fn assert(text: impl Into<T>) -> Self {
        Text::try_from_string(text).expect("failed to construct morpheme-encoded text")
    }

    pub const fn empty() -> Self
    where
        T: Empty,
    {
        Text {
            text: T::EMPTY,
            _phantom: PhantomData,
        }
    }

    pub fn try_map_string<U, F>(self, f: F) -> Result<Text<U, M>, MorphologyError>
    where
        U: AsRef<str> + Unchecked + Unicode,
        F: FnOnce(T) -> U,
    {
        let Text { text, .. } = self;
        Text::try_from_string(f(text))
    }
}

impl<T, M> Text<T, M>
where
    T: AsRef<str>,
{
    pub fn as_str(&self) -> &str {
        AsRef::<str>::as_ref(self)
    }
}

impl<'t, M> Text<Cow<'t, str>, M> {
    pub fn into_owned(self) -> Text<Cow<'static, str>, M> {
        let Text { text, .. } = self;
        Text {
            text: text.into_owned().into(),
            _phantom: PhantomData,
        }
    }
}

impl<T, M> Text<T, M>
where
    T: AsRef<str> + Unchecked + Unicode,
    M: MorphemeFamily,
{
    pub fn segments<S>(&self) -> impl '_ + Iterator<Item = Segment<&'_ str, M, S>>
    where
        S: Default,
    {
        self.text
            .as_ref()
            .split_at_ascii_line_breaks()
            .map(|text| Segment::unchecked(Text::unchecked(text), S::default()))
    }

    pub fn segments_with_transform<'s, S>(
        &'s self,
        transform: S,
    ) -> impl 's + Iterator<Item = Segment<&'s str, M, S>>
    where
        S: 's + Clone,
    {
        self.segments_with(move || transform.clone())
    }

    // TODO: Without a consuming split function, it is impossible to consume `Text` and yield
    //       `Segments` (without cloning the data and other awkward API limitations).
    //
    //       Eh, this requires allocation for text representations where lines are not structural
    //       (i.e., any bog standard string... so basically everything). Just clone/allocate. It'll
    //       have to happen anyway.
    pub fn segments_with<'s, S, F>(
        &'s self,
        mut f: F,
    ) -> impl 's + Iterator<Item = Segment<&'s str, M, S>>
    where
        F: 's + FnMut() -> S,
    {
        self.text
            .as_ref()
            .split_at_ascii_line_breaks()
            .map(move |text| Segment::unchecked(Text::unchecked(text), f()))
    }

    // This indirection is very intentional. `Text` is non-structural w.r.t. lines: it is just text
    // and line breaks are encoded inline as code points. In other words, it is not designed for
    // block layout. This function is an **explicit** way to interpret it as such (and only through
    // a borrow).
    pub fn as_block_layout(&self) -> impl '_ + BlockLayout {
        #[derive(Debug)]
        struct AsBlockLayout<'b, T, M>(&'b Text<T, M>);

        impl<T, M> BlockLayout for AsBlockLayout<'_, T, M>
        where
            T: AsRef<str> + Unicode,
            M: MorphemeFamily,
        {
            fn ascii_line_break_bounds(&self) -> BoundingBox {
                let mut count = 0;
                let width = self
                    .0
                    .text
                    .as_ref()
                    .split_at_ascii_line_breaks()
                    .enumerate()
                    .map(|(index, line)| {
                        count = index;
                        line
                    })
                    .map(|line| line.width())
                    .max()
                    .unwrap_or(0);
                BoundingBox {
                    width,
                    // SAFETY: The input `usize` is never zero, because it is the result of a
                    //         saturating addition with one.
                    height: unsafe { NonZeroUsize::new_unchecked(count.saturating_add(1)) },
                }
            }
        }

        impl<T, M> Encoded for AsBlockLayout<'_, T, M>
        where
            T: Unicode,
            M: MorphemeFamily,
        {
            type MorphemeFamily = M;
        }

        impl<T, M> Unicode for AsBlockLayout<'_, T, M>
        where
            T: Unicode,
        {
            fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<Grapheme<'_>>> {
                self.0.graphemes()
            }

            fn width(&self) -> usize {
                self.0.width()
            }
        }

        AsBlockLayout(self)
    }
}

impl<T, M> ops::Append for Text<T, M>
where
    T: AsRef<str> + ToStringMut,
    M: MorphemeFamily,
{
    fn append(mut self, rhs: Self) -> Self {
        self.text.to_string_mut().push_str(rhs.as_str());
        self
    }
}

impl<T, M> AsRef<str> for Text<T, M>
where
    T: AsRef<str>,
{
    fn as_ref(&self) -> &str {
        self.text.as_ref()
    }
}

impl<T, M> Encoded for Text<T, M>
where
    T: Unicode,
    M: MorphemeFamily,
{
    type MorphemeFamily = M;
}

impl<T, M> ops::Extend for Text<T, M>
where
    T: ToStringMut,
    M: MorphemeFamily,
{
    fn extend<'t, I>(&mut self, morphemes: I) -> usize
    where
        I: IntoIterator<Item = MorphemeFor<'t, Self::MorphemeFamily>>,
    {
        self.text
            .to_string_mut()
            .extend(morphemes.into_iter().map(Morpheme::into_string));
        self.width()
    }
}

impl<T, M> LinearLayout for Text<T, M>
where
    T: Unicode,
    M: MorphemeFamily,
{
}

impl<T, M> ops::Truncate for Text<T, M>
where
    T: ToStringMut,
    M: MorphemeFamily,
{
    fn truncate(&mut self, max: usize) -> usize {
        let mut width = 0usize;
        // This iterator expression cannot use `find`, because it borrows the iterator, which
        // prevents the mutable borrow in the branch.
        if let Some(len) = self
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

impl<T, M> Unicode for Text<T, M>
where
    T: Unicode,
{
    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<Grapheme<'_>>> {
        self.text.graphemes()
    }

    fn width(&self) -> usize {
        self.text.width()
    }
}

// TODO: Segments ignore non-ASCII line breaks (by design). Make sure this is documented.
// TODO: Consider `unicode-linebreak` or something similar if it seems that support for line
//       breaking Unicode control characters like LS and PS is justified.
// TODO: The derived implementations do not depend on the type parameter `M`. Implement them
//       explicitly to reflect this.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Segment<T, M = FlexFamily, S = ()> {
    text: Text<T, M>,
    transform: S,
}

impl<T, M, S> Segment<T, M, S>
where
    T: AsRef<str> + Unchecked + Unicode,
    M: MorphemeFamily,
{
    const fn unchecked(text: Text<T, M>, transform: S) -> Self {
        Segment { text, transform }
    }

    pub fn try_from_text_with_transform(
        text: Text<T, M>,
        transform: S,
    ) -> Result<Self, ControlError> {
        if text.as_ref().has_ascii_line_breaks() {
            Err(ControlError)
        }
        else {
            Ok(Segment::unchecked(text, transform))
        }
    }

    pub fn from_text_or_joined_with_transform(text: Text<T, M>, transform: S) -> Self
    where
        T: From<String>,
    {
        let mut lines = text.as_str().split_at_ascii_line_breaks().peekable();
        let first = lines.next();
        let text = if lines.peek().is_some() {
            Text::unchecked(first.into_iter().chain(lines).join("").into())
        }
        else {
            drop(lines);
            text
        };
        Segment { text, transform }
    }

    pub fn from_text_or_joined(text: Text<T, M>) -> Self
    where
        T: From<String>,
        S: Default,
    {
        Segment::from_text_or_joined_with_transform(text, S::default())
    }

    pub fn map_transform<U, F>(self, f: F) -> Segment<T, M, U>
    where
        F: FnOnce(S) -> U,
    {
        let Segment { text, transform } = self;
        Segment {
            text,
            transform: f(transform),
        }
    }
}

impl<T, M, S> Segment<T, M, S>
where
    T: AsRef<str>,
{
    pub fn as_str(&self) -> &str {
        AsRef::<str>::as_ref(self)
    }
}

impl<T, M> Segment<T, M, ()>
where
    T: AsRef<str> + Unchecked + Unicode,
    M: MorphemeFamily,
{
    pub const fn empty() -> Self
    where
        T: Empty,
    {
        Segment::unchecked(Text::empty(), ())
    }
}

impl<'t, M, S> Segment<Cow<'t, str>, M, S> {
    pub fn into_owned(self) -> Segment<Cow<'static, str>, M, S> {
        let Segment { text, transform } = self;
        Segment {
            text: text.into_owned(),
            transform,
        }
    }
}

impl<T, M, S> AsRef<str> for Segment<T, M, S>
where
    T: AsRef<str>,
{
    fn as_ref(&self) -> &str {
        self.text.as_ref()
    }
}

impl<T, M, S> BlockLayout for Segment<T, M, S>
where
    T: Unicode,
    M: MorphemeFamily,
{
    fn ascii_line_break_bounds(&self) -> BoundingBox {
        BoundingBox {
            width: self.width(),
            // This type explicitly rejects any and all line breaks.
            height: NonZeroUsize::ONE,
        }
    }
}

impl<T, M, S> Encoded for Segment<T, M, S>
where
    T: Unicode,
    M: MorphemeFamily,
{
    type MorphemeFamily = M;
}

impl<T, M, S> LinearLayout for Segment<T, M, S>
where
    T: Unicode,
    M: MorphemeFamily,
{
}

impl<T, M, S> TryFrom<Text<T, M>> for Segment<T, M, S>
where
    T: AsRef<str> + Unchecked + Unicode,
    M: MorphemeFamily,
    S: Default,
{
    type Error = ControlError;

    fn try_from(text: Text<T, M>) -> Result<Self, Self::Error> {
        Segment::try_from_text_with_transform(text, S::default())
    }
}

impl<T, M, S> Unicode for Segment<T, M, S>
where
    T: Unicode,
{
    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<Grapheme<'_>>> {
        self.text.graphemes()
    }

    fn width(&self) -> usize {
        self.text.width()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SegmentedIndex {
    pub segment: usize,
    pub byte: usize,
}

// TODO: The derived implementations do not depend on the type parameter `M`. Implement them
//       explicitly to reflect this.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Line<T, M = FlexFamily, S = ()> {
    // TODO: Perhaps segments ought to be stored in a `VecDeque` instead? If prepending becomes
    //       necessary in code written against `Line`, consider making this change.
    segments: Vec<Segment<T, M, S>>,
}

impl<T, M, S> Line<T, M, S> {
    pub const fn empty() -> Self {
        Line {
            segments: Vec::new(),
        }
    }

    pub fn push(&mut self, segment: impl Into<Segment<T, M, S>>) {
        self.segments.push(segment.into());
    }

    pub fn segments(&self) -> &[Segment<T, M, S>] {
        self.segments.as_slice()
    }
}

impl<T, M, S> Line<T, M, S>
where
    T: AsRef<str>,
{
    pub fn to_string(&self) -> Cow<'_, str> {
        let segments = self.segments();
        match segments.len() {
            0 => "".into(),
            1 => segments.get(0).unwrap().as_str().into(),
            _ => segments.iter().map(Segment::as_str).join("").into(),
        }
    }
}

impl<T, M, S> Line<T, M, S>
where
    T: Unicode,
{
    // TODO: Hmm, this seems like the index type ought to be an associated type for `graphemes` and
    //       `morphemes`, rather than using a bespoke function here. The `usize` index in
    //       `Line::graphemes` is basically nonsense!
    pub fn segmentation(
        &self,
    ) -> impl '_ + Clone + Iterator<Item = Indexed<Grapheme<'_>, SegmentedIndex>> {
        self.segments
            .iter()
            .enumerate()
            .flat_map(|(index, segment)| {
                segment.graphemes().map(move |grapheme| Indexed {
                    index: SegmentedIndex {
                        segment: index,
                        byte: grapheme.index,
                    },
                    text: grapheme.text,
                })
            })
    }
}

impl<'t, M, S> Line<Cow<'t, str>, M, S> {
    pub fn into_owned(self) -> Line<Cow<'static, str>, M, S> {
        let Line { segments } = self;
        Line {
            segments: segments.into_iter().map(Segment::into_owned).collect(),
        }
    }
}

impl<T, M, S> BlockLayout for Line<T, M, S>
where
    T: Unicode,
    M: MorphemeFamily,
{
    fn ascii_line_break_bounds(&self) -> BoundingBox {
        BoundingBox {
            width: self.width(),
            // This type explicitly rejects any and all line breaks.
            height: NonZeroUsize::ONE,
        }
    }
}

impl<T, M, S> Default for Line<T, M, S> {
    fn default() -> Self {
        Line {
            segments: Default::default(),
        }
    }
}

impl<T, M, S> Encoded for Line<T, M, S>
where
    T: Unicode,
    M: MorphemeFamily,
{
    type MorphemeFamily = M;
}

impl<T, M, S> From<Vec<Segment<T, M, S>>> for Line<T, M, S> {
    fn from(segments: Vec<Segment<T, M, S>>) -> Self {
        Line { segments }
    }
}

impl<T, M, S> FromIterator<Segment<T, M, S>> for Line<T, M, S> {
    fn from_iter<I>(input: I) -> Self
    where
        I: IntoIterator<Item = Segment<T, M, S>>,
    {
        Line::from(input.into_iter().collect::<Vec<_>>())
    }
}

impl<T, M, S> LinearLayout for Line<T, M, S>
where
    T: Unicode,
    M: MorphemeFamily,
{
}

impl<T, M, S> Unicode for Line<T, M, S>
where
    T: Unicode,
{
    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<Grapheme<'_>>> {
        self.segments.as_slice().graphemes()
    }

    fn width(&self) -> usize {
        self.segments.as_slice().width()
    }
}

fn is_ascii_line_break(byte: u8) -> bool {
    matches!(byte, CR | LF)
}

fn annex11_point_width_ambiguous_non_cjk(point: char) -> usize {
    use unicode_width::UnicodeWidthChar;

    UnicodeWidthChar::width(point).unwrap_or(0)
}

// Here, "ambiguous non-CJK" means that UAX11 ambiguous graphemes are assigned the "non-CJK" column
// width of one (rather than two, which is typically more compatible in CJK contexts). Generally,
// ambiguous and halfwidth graphemes are both mapped to narrow morphemes and are treated the same.
fn annex11_text_width_ambiguous_non_cjk(text: &str) -> usize {
    use unicode_width::UnicodeWidthStr;

    // NOTE: This considers some potentially troublesome ASCII whitespace characters as zero-width,
    //       which works well here! For example, TAB is zero-width and so is not a morpheme.
    UnicodeWidthStr::width(text)
}

fn annex29_text_grapheme_segmentation(
    text: &str,
) -> impl '_ + Clone + Iterator<Item = Indexed<Grapheme<'_>>> {
    use unicode_segmentation::UnicodeSegmentation;

    UnicodeSegmentation::grapheme_indices(text, true)
        .map(Indexed::from)
        .map(|grapheme| grapheme.map(Cow::from).map(Grapheme::unchecked))
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
