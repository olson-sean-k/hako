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

    pub fn assert<T>(text: T) -> Self
    where
        Self: TryFrom<T>,
        <Self as TryFrom<T>>::Error: Debug,
    {
        Text::try_from(text).expect("failed to construct morpheme-encoded text")
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

    pub fn try_map_string<F>(self, f: F) -> Result<Self, MorphologyError>
    where
        F: FnOnce(Cow<'t, str>) -> Cow<'t, str>,
    {
        let Text { text, .. } = self;
        Text::try_from(f(text))
    }

    pub fn segments<S>(&self) -> impl '_ + Iterator<Item = Segment<'_, M, S>>
    where
        S: Default,
    {
        self.text
            .split_at_ascii_line_breaks()
            .map(|text| Segment::unchecked(Text::unchecked(text.into()), S::default()))
    }

    pub fn segments_with_transform<'s, S>(
        &'s self,
        transform: S,
    ) -> impl 's + Iterator<Item = Segment<'s, M, S>>
    where
        S: 's + Clone,
    {
        self.segments_with(move || transform.clone())
    }

    // TODO: Without a consuming split function, it is impossible to consume `Text` and yield
    //       `Segments` (without cloning the data and other awkward API limitations).
    pub fn segments_with<'s, S, F>(
        &'s self,
        mut f: F,
    ) -> impl 's + Iterator<Item = Segment<'s, M, S>>
    where
        F: 's + FnMut() -> S,
    {
        self.text
            .split_at_ascii_line_breaks()
            .map(move |text| Segment::unchecked(Text::unchecked(text.into()), f()))
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

// TODO: Segments ignore non-ASCII line breaks (by design). Make sure this is documented.
// TODO: Consider `unicode-linebreak` or something similar if it seems that support for line
//       breaking Unicode control characters like LS and PS is justified.
// TODO: The derived implementations do not depend on the type parameter `M`. Implement them
//       explicitly to reflect this.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Segment<'t, M = Flex<'t>, S = ()> {
    text: Text<'t, M>,
    transform: S,
}

impl<'t, M, S> Segment<'t, M, S> {
    const fn unchecked(text: Text<'t, M>, transform: S) -> Self {
        Segment { text, transform }
    }

    pub fn try_from_text_with_transform(
        text: Text<'t, M>,
        transform: S,
    ) -> Result<Self, ControlError> {
        if text.as_ref().has_ascii_line_breaks() {
            Err(ControlError)
        }
        else {
            Ok(Segment::unchecked(text, transform))
        }
    }

    pub fn from_text_or_joined_with_transform(text: Text<'t, M>, transform: S) -> Self {
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

    pub fn from_text_or_joined(text: Text<'t, M>) -> Self
    where
        S: Default,
    {
        Segment::from_text_or_joined_with_transform(text, S::default())
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

impl<'t, M> Segment<'t, M, ()> {
    pub const fn empty() -> Self {
        Segment::unchecked(Text::empty(), ())
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

impl<'t, M, S> TryFrom<Text<'t, M>> for Segment<'t, M, S>
where
    S: Default,
{
    type Error = ControlError;

    fn try_from(text: Text<'t, M>) -> Result<Self, Self::Error> {
        Segment::try_from_text_with_transform(text, S::default())
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
    pub const fn empty() -> Self {
        Line {
            segments: Vec::new(),
        }
    }

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

impl<M, S> Default for Line<'_, M, S> {
    fn default() -> Self {
        Line {
            segments: Default::default(),
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

fn is_ascii_line_break(byte: u8) -> bool {
    matches!(byte, CR | LF)
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
