pub mod ops;

use itertools::Itertools;
use std::borrow::Cow;
use std::convert::Infallible;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::num::NonZeroUsize;

use crate::breadth::Breadth;
use crate::slice::SliceProjection;

#[derive(Clone, Copy, Debug)]
pub struct MorphologyError;

impl From<Infallible> for MorphologyError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

pub trait Unicode {
    fn graphemes(&self) -> impl '_ + Iterator<Item = Grapheme<'_>>;

    fn width(&self) -> usize;
}

impl<T> Unicode for T
where
    T: ?Sized + SliceProjection,
    T::Item: Unicode,
{
    fn graphemes(&self) -> impl '_ + Iterator<Item = Grapheme<'_>> {
        self.iter().flat_map(|unicode| unicode.graphemes())
    }

    fn width(&self) -> usize {
        self.iter().map(|unicode| unicode.width()).sum()
    }
}

impl Unicode for str {
    fn graphemes(&self) -> impl '_ + Iterator<Item = Grapheme<'_>> {
        annex29_text_grapheme_segmentation(self)
    }

    fn width(&self) -> usize {
        annex11_text_width_ambiguous_non_cjk(self)
    }
}

impl Unicode for char {
    fn graphemes(&self) -> impl '_ + Iterator<Item = Grapheme<'_>> {
        Some(Grapheme::from(*self)).into_iter()
    }

    fn width(&self) -> usize {
        annex11_point_width_ambiguous_non_cjk(*self)
    }
}

impl<'t> Unicode for Cow<'t, str> {
    fn graphemes(&self) -> impl '_ + Iterator<Item = Grapheme<'_>> {
        self.as_ref().graphemes()
    }

    fn width(&self) -> usize {
        self.as_ref().width()
    }
}

impl Unicode for String {
    fn graphemes(&self) -> impl '_ + Iterator<Item = Grapheme<'_>> {
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
    fn graphemes(&self) -> impl '_ + Iterator<Item = Grapheme<'_>> {
        self.as_ref().graphemes()
    }

    fn width(&self) -> usize {
        self.as_ref().width()
    }
}

pub trait Encoded: Unicode {
    type Morpheme: Morpheme;

    fn morphemes(&self) -> impl '_ + Iterator<Item = Annex11<'_, Self::Morpheme>> {
        self.graphemes()
            .map(Annex11::<Self::Morpheme>::try_from)
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
    type Morpheme = <T::Item as Encoded>::Morpheme;
}

pub trait Morpheme {
    type Annex11<'t>: Into<Flex<'t>> + TryFrom<Grapheme<'t>>;
}

pub type Annex11<'t, M> = <M as Morpheme>::Annex11<'t>;

pub type Flex<'t> = Breadth<Narrow<'t>, Wide<'t>>;

impl<'t> Flex<'t> {
    pub fn into_owned(self) -> Flex<'t> {
        match self {
            Flex::Narrow(narrow) => Flex::Narrow(narrow.into_owned()),
            Flex::Wide(wide) => Flex::Wide(wide.into_owned()),
        }
    }

    pub fn width(&self) -> NonZeroUsize {
        match self {
            Flex::Narrow(_) => Narrow::WIDTH,
            Flex::Wide(_) => Wide::WIDTH,
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

impl Morpheme for Flex<'_> {
    type Annex11<'t> = Flex<'t>;
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
    // SAFETY: This must not be constructed from `0usize`.
    pub const WIDTH: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(1) };

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

impl Morpheme for Narrow<'_> {
    type Annex11<'t> = Narrow<'t>;
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
    fn graphemes(&self) -> impl '_ + Iterator<Item = Grapheme<'_>> {
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
    // SAFETY: This must not be constructed from `0usize`.
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

impl Morpheme for Wide<'_> {
    type Annex11<'t> = Wide<'t>;
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
    fn graphemes(&self) -> impl '_ + Iterator<Item = Grapheme<'_>> {
        self.grapheme.graphemes()
    }

    #[inline(always)]
    fn width(&self) -> usize {
        Wide::WIDTH.into()
    }
}

// TODO: The derived implementations do not depend on the type parameter `M`. Implement them
//       explicitly to reflect this.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct Text<'t, M = Flex<'t>> {
    text: Cow<'t, str>,
    _phantom: PhantomData<fn() -> M>,
}

impl<'t, M> Text<'t, M> {
    fn unchecked(text: Cow<'t, str>) -> Self {
        Text {
            text,
            _phantom: PhantomData,
        }
    }

    pub const fn empty() -> Text<'static, M> {
        Text {
            text: Cow::Borrowed(""),
            _phantom: PhantomData,
        }
    }

    pub fn into_owned(self) -> Text<'static, M> {
        let Text { text, .. } = self;
        Text {
            text: text.into_owned().into(),
            _phantom: PhantomData,
        }
    }

    pub fn as_str(&self) -> &str {
        AsRef::<str>::as_ref(self)
    }
}

impl<'t, M> Text<'t, M>
where
    M: Morpheme,
{
    pub fn from_string_or_empty(text: impl Into<Cow<'t, str>>) -> Self {
        match Text::try_from(text.into()) {
            Ok(text) => text,
            _ => Text::empty(),
        }
    }
}

impl<'t, M> AsRef<str> for Text<'t, M> {
    fn as_ref(&self) -> &str {
        self.text.as_ref()
    }
}

impl<M> Encoded for Text<'_, M>
where
    M: Morpheme,
{
    type Morpheme = M;
}

impl<'t, M> TryFrom<Cow<'t, str>> for Text<'t, M>
where
    M: Morpheme,
{
    type Error = MorphologyError;

    fn try_from(text: Cow<'t, str>) -> Result<Self, Self::Error> {
        use std::mem;

        // TODO: Lifetimes are overcaptured in `Unicode::graphemes`. In this case, the lifetime
        //       `'t` is captured and so the borrow through `graphemes` persists even after
        //       dropping the iterator. The text is coerced to `&'static str` to examine the
        //       graphemes instead. This is far from ideal, but abstracts over ownership without
        //       unnecessary cloning.
        //
        //       See https://rust-lang.github.io/rfcs/3498-lifetime-capture-rules-2024.html#overcapturing
        // SAFETY: The transmuted `non_static_inner_text` binding must not escape this function in
        //         any way: it is **not** `'static` and is only valid over the borrow of `text` in
        //         `AsRef::as_ref` (local to this function).
        let non_static_inner_text =
            unsafe { mem::transmute::<&'_ str, &'static str>(text.as_ref()) };
        if non_static_inner_text
            .graphemes()
            .map(Annex11::<M>::try_from)
            .all(|morpheme| morpheme.is_ok())
        {
            Ok(Text::unchecked(text))
        }
        else {
            Err(MorphologyError)
        }
    }
}

impl<'t, M> TryFrom<&'t str> for Text<'t, M>
where
    M: Morpheme,
{
    type Error = MorphologyError;

    fn try_from(text: &'t str) -> Result<Self, Self::Error> {
        Cow::from(text).try_into()
    }
}

impl<'t, M> TryFrom<String> for Text<'t, M>
where
    M: Morpheme,
{
    type Error = MorphologyError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Cow::from(text).try_into()
    }
}

impl<'t, M> Unicode for Text<'t, M> {
    fn graphemes(&self) -> impl '_ + Iterator<Item = Grapheme<'_>> {
        self.as_ref().graphemes()
    }

    fn width(&self) -> usize {
        self.as_ref().width()
    }
}

// TODO: The derived implementations do not depend on the type parameter `M`. Implement them
//       explicitly to reflect this.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Segment<'t, M = Flex<'t>, S = ()> {
    text: Text<'t, M>,
    transform: S,
}

impl<'t, M, S> Segment<'t, M, S> {
    pub fn from_text_transform(text: Text<'t, M>, transform: S) -> Self {
        let mut lines = text.as_str().lines().peekable();
        let first = lines.next();
        let text = if lines.peek().is_some() {
            Text::unchecked(first.into_iter().chain(lines).join("").into())
        }
        else {
            text
        };
        Segment { text, transform }
    }

    pub fn from_text(text: Text<'t, M>) -> Self
    where
        S: Default,
    {
        Segment::from_text_transform(text, S::default())
    }

    pub fn into_owned(self) -> Segment<'static, M, S> {
        let Segment { text, transform } = self;
        Segment {
            text: text.into_owned(),
            transform,
        }
    }

    pub fn map_transform<U, F>(self, f: F) -> Segment<'t, M, U>
    where
        F: FnOnce(S) -> U,
    {
        let Segment { text, transform } = self;
        Segment {
            text,
            transform: f(transform),
        }
    }

    pub fn as_str(&self) -> &str {
        AsRef::<str>::as_ref(self)
    }
}

impl<M, S> AsRef<str> for Segment<'_, M, S> {
    fn as_ref(&self) -> &str {
        self.text.as_ref()
    }
}

impl<M, S> Encoded for Segment<'_, M, S>
where
    M: Morpheme,
{
    type Morpheme = M;
}

impl<'t, M, S> From<Text<'t, M>> for Segment<'t, M, S>
where
    S: Default,
{
    fn from(text: Text<'t, M>) -> Self {
        Segment::from_text(text)
    }
}

impl<M, S> Unicode for Segment<'_, M, S> {
    fn graphemes(&self) -> impl '_ + Iterator<Item = Grapheme<'_>> {
        self.text.graphemes()
    }

    fn width(&self) -> usize {
        self.text.width()
    }
}

// TODO: The derived implementations do not depend on the type parameter `M`. Implement them
//       explicitly to reflect this.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Line<'t, M = Flex<'t>, S = ()> {
    // TODO: Perhaps segments ought to be stored in a `VecDeque` instead? If prepending becomes
    //       necessary in code written against `Line`, consider making this change.
    segments: Vec<Segment<'t, M, S>>,
}

impl<'t, M, S> Line<'t, M, S> {
    pub fn into_owned(self) -> Line<'static, M, S> {
        let Line { segments } = self;
        Line {
            segments: segments.into_iter().map(Segment::into_owned).collect(),
        }
    }

    pub fn push(&mut self, segment: impl Into<Segment<'t, M, S>>) {
        self.segments.push(segment.into());
    }

    pub fn segments(&self) -> &[Segment<'t, M, S>] {
        self.segments.as_slice()
    }

    pub fn to_string(&self) -> Cow<'_, str> {
        let segments = self.segments();
        match segments.len() {
            0 => "".into(),
            1 => segments.get(0).unwrap().as_str().into(),
            _ => segments.iter().map(Segment::as_str).join("").into(),
        }
    }
}

impl<M, S> Encoded for Line<'_, M, S>
where
    M: Morpheme,
{
    type Morpheme = M;

    fn morphemes(&self) -> impl '_ + Iterator<Item = Annex11<'_, Self::Morpheme>> {
        self.segments.as_slice().morphemes()
    }
}

impl<'t, M, S> From<Vec<Segment<'t, M, S>>> for Line<'t, M, S> {
    fn from(segments: Vec<Segment<'t, M, S>>) -> Self {
        Line { segments }
    }
}

impl<'t, M, S> FromIterator<Segment<'t, M, S>> for Line<'t, M, S> {
    fn from_iter<I>(input: I) -> Self
    where
        I: IntoIterator<Item = Segment<'t, M, S>>,
    {
        Line::from(input.into_iter().collect::<Vec<_>>())
    }
}

impl<M, S> Unicode for Line<'_, M, S> {
    fn graphemes(&self) -> impl '_ + Iterator<Item = Grapheme<'_>> {
        self.segments.as_slice().graphemes()
    }

    fn width(&self) -> usize {
        self.segments.as_slice().width()
    }
}

fn annex11_point_width_ambiguous_non_cjk(point: char) -> usize {
    use unicode_width::UnicodeWidthChar;

    UnicodeWidthChar::width(point).unwrap_or(0)
}

fn annex11_text_width_ambiguous_non_cjk(text: &str) -> usize {
    use unicode_width::UnicodeWidthStr;

    UnicodeWidthStr::width(text)
}

fn annex29_text_grapheme_segmentation(text: &str) -> impl '_ + Iterator<Item = Grapheme<'_>> {
    use unicode_segmentation::UnicodeSegmentation;

    UnicodeSegmentation::graphemes(text, true)
        .map(Cow::from)
        .map(Grapheme::unchecked)
}
