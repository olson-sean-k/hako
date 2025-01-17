use derive_where::derive_where;
use itertools::Itertools;
use std::borrow::Cow;
use std::convert::Infallible;
use std::fmt::{self, Display, Formatter};
use std::iter;
use std::marker::PhantomData;

use crate::cow::MoveCow;
use crate::text::geometry::LinearGeometry;
use crate::text::modal::ModalText;
use crate::text::morphology::{FlexKind, Grapheme, Morpheme, MorphemeFor, MorphemeKind};
use crate::text::render::{AsDisplay, FmtWith, Render, RenderContext};
use crate::text::style::AnsiPrefix;
use crate::text::{
    ops, BlankText, BlockText, BlockTextProjection, Indexed, MorphologyError, RawText, StrExt as _,
    ToStringMut, TryFromText,
};

use ModalText::{Blank, Content};

pub type SegmentFor<T, M> = Segment<<T as BlockTextProjection>::RawText, M>;

#[derive_where(Clone, Copy, Debug, Eq, Hash, PartialEq; T)]
pub struct Segment<T = String, M = FlexKind> {
    modal: ModalSegment<T, M>,
}

impl<T, M> Segment<T, M>
where
    T: RawText,
    M: MorphemeKind,
{
    pub fn to_string(&self) -> Cow<'_, str> {
        match self.modal {
            Blank(ref blank) => blank.to_string().into(),
            Content(ref content) => content.as_ref().into(),
        }
    }

    pub fn display(&self) -> impl '_ + Display {
        AsDisplay::<_, ()>::from(self)
    }

    pub fn is_blank(&self) -> bool {
        match self.modal {
            Blank(_) => true,
            Content(ref content) => content.is_blank(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self.modal {
            Blank(ref blank) => blank.is_empty(),
            Content(ref content) => content.is_empty(),
        }
    }
}

impl<'t, M> Segment<Cow<'t, str>, M>
where
    M: MorphemeKind,
{
    pub fn into_owned(self) -> Segment<Cow<'static, str>, M> {
        match self.modal {
            Blank(blank) => blank.into_owned().into(),
            Content(content) => content.into_owned().into(),
        }
    }
}

impl<T, M> ops::Append for Segment<T, M>
where
    T: From<String> + RawText + ToStringMut,
    M: MorphemeKind,
{
    fn append(self, rhs: Self) -> Self {
        match (self.modal, rhs.modal) {
            (Blank(lhs), Blank(rhs)) => lhs.append(rhs).into(),
            (Content(lhs), Content(rhs)) => lhs.append(rhs).into(),
            (Blank(lhs), Content(rhs)) => ContentSegment::from(lhs).append(rhs).into(),
            (Content(lhs), Blank(rhs)) => lhs.append(ContentSegment::from(rhs)).into(),
        }
    }
}

impl<T, M> BlockText for Segment<T, M>
where
    T: RawText,
    M: MorphemeKind,
{
    type RawText = T;
    type Morpheme<'t>
        = MorphemeFor<'t, M>
    where
        Self: 't;
    type Index = usize;

    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<Self::Index, Grapheme<'_>>> {
        self.modal
            .as_ref()
            .map_blank(BlankSegment::graphemes)
            .map_content(ContentSegment::graphemes)
    }

    fn morphemes(
        &self,
    ) -> impl '_ + Clone + Iterator<Item = Indexed<Self::Index, Self::Morpheme<'_>>> {
        self.modal
            .as_ref()
            .map_blank(BlankSegment::morphemes)
            .map_content(ContentSegment::morphemes)
    }
}

impl<T, M> From<BlankSegment<T, M>> for Segment<T, M> {
    fn from(segment: BlankSegment<T, M>) -> Self {
        Segment {
            modal: Blank(segment),
        }
    }
}

impl<T, M> From<ContentSegment<T, M>> for Segment<T, M> {
    fn from(segment: ContentSegment<T, M>) -> Self {
        Segment {
            modal: Content(segment),
        }
    }
}

impl<T, M> LinearGeometry for Segment<T, M>
where
    T: RawText,
    M: MorphemeKind,
{
    fn width(&self) -> usize {
        match self.modal {
            Blank(ref blank) => blank.width(),
            Content(ref content) => content.width(),
        }
    }
}

impl<T, M, S> Render<S> for Segment<T, M>
where
    T: RawText,
    M: MorphemeKind,
    S: AnsiPrefix,
{
    fn fmt(&self, formatter: &mut Formatter, context: &mut RenderContext<S>) -> fmt::Result {
        match self.modal {
            Blank(ref blank) => Render::fmt(blank, formatter, context),
            Content(ref content) => Render::fmt(content, formatter, context),
        }
    }
}

impl<T, M> TryFromText<BlankText> for Segment<T, M>
where
    T: RawText,
    M: MorphemeKind,
{
    type Error = MorphologyError;

    fn try_from_text(text: BlankText) -> Result<Self, Self::Error> {
        BlankSegment::try_from_text(text).map(From::from)
    }
}

impl<T, M> TryFromText<Segment<T, M>> for Segment<T, M> {
    type Error = Infallible;

    fn try_from_text(segment: Segment<T, M>) -> Result<Self, Self::Error> {
        Ok(segment)
    }
}

impl<T, U, M> TryFromText<U> for Segment<T, M>
where
    T: RawText + TryFrom<MoveCow<U>>,
    U: RawText,
    M: MorphemeKind,
{
    type Error = MorphologyError;

    fn try_from_text(text: U) -> Result<Self, Self::Error> {
        ContentSegment::try_from_text(text).map(From::from)
    }
}

type ModalSegment<T, M> = ModalText<BlankSegment<T, M>, ContentSegment<T, M>>;

#[derive_where(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct BlankSegment<T = String, M = FlexKind> {
    width: usize,
    _phantom: PhantomData<fn() -> (T, M)>,
}

impl<T, M> BlankSegment<T, M>
where
    M: MorphemeKind,
{
    pub const fn empty() -> Self {
        BlankSegment::from_width_unchecked(0)
    }

    const fn from_width_unchecked(width: usize) -> Self {
        BlankSegment {
            width,
            _phantom: PhantomData,
        }
    }

    pub const fn try_from_width(width: usize) -> Result<Self, MorphologyError> {
        if width % M::MIN_WIDTH.get() == 0 {
            Ok(BlankSegment::from_width_unchecked(width))
        }
        else {
            Err(MorphologyError)
        }
    }

    pub fn from_min_width(width: usize) -> Self {
        BlankSegment::from_width_unchecked(BlankText::from_min_width::<M>(width).into())
    }

    pub fn from_max_width(width: usize) -> Self {
        BlankSegment::from_width_unchecked(BlankText::from_max_width::<M>(width).into())
    }

    pub fn from_min_width_morpheme_count(n: usize) -> Self {
        BlankSegment::from_width_unchecked(BlankText::from_min_width_morpheme_count::<M>(n).into())
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0
    }
}

impl<T, M> BlankSegment<T, M>
where
    T: RawText,
    M: MorphemeKind,
{
    pub fn to_string(&self) -> String {
        self.morphemes()
            .map(Indexed::into_text)
            .map(Morpheme::into_string)
            .join("")
    }
}

impl<'t, M> BlankSegment<Cow<'t, str>, M>
where
    M: MorphemeKind,
{
    pub fn into_owned(self) -> BlankSegment<Cow<'static, str>, M> {
        BlankSegment::from_width_unchecked(self.width)
    }
}

impl<T, M> ops::Append for BlankSegment<T, M>
where
    T: RawText,
    M: MorphemeKind,
{
    fn append(self, rhs: Self) -> Self {
        BlankSegment::from_width_unchecked(
            self.width
                .checked_add(rhs.width)
                .expect("overflow appending text"),
        )
    }
}

impl<T, M> BlockText for BlankSegment<T, M>
where
    T: RawText,
    M: MorphemeKind,
{
    type RawText = T;
    type Morpheme<'t>
        = MorphemeFor<'t, M>
    where
        Self: 't;
    type Index = usize;

    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<Self::Index, Grapheme<'_>>> {
        self.morphemes()
            .map(|morpheme| morpheme.map_text(Into::into))
    }

    fn morphemes(
        &self,
    ) -> impl '_ + Clone + Iterator<Item = Indexed<Self::Index, Self::Morpheme<'_>>> {
        iter::repeat(M::min_width_blank())
            .enumerate()
            .take(self.width / M::MIN_WIDTH)
            .map(move |(index, text)| Indexed { index, text })
    }
}

impl<T, M> LinearGeometry for BlankSegment<T, M>
where
    T: RawText,
    M: MorphemeKind,
{
    fn width(&self) -> usize {
        self.width
    }
}

impl<T, M, S> Render<S> for BlankSegment<T, M>
where
    T: RawText,
    M: MorphemeKind,
    S: AnsiPrefix,
{
    fn fmt(&self, formatter: &mut Formatter, context: &mut RenderContext<S>) -> fmt::Result {
        context.fmt_with_ansi_fence(
            formatter,
            FmtWith(|formatter| {
                for morpheme in self.morphemes().map(Indexed::into_text) {
                    write!(formatter, "{}", morpheme.as_ref())?;
                }
                Ok(())
            }),
        )
    }
}

impl<T, M> ops::Truncate for BlankSegment<T, M>
where
    T: RawText,
    M: MorphemeKind,
{
    fn truncate(&mut self, max: usize) -> usize {
        let segment = BlankSegment::from_max_width(max);
        *self = segment;
        segment.width()
    }
}

impl<T, M> TryFromText<BlankSegment<T, M>> for BlankSegment<T, M> {
    type Error = Infallible;

    fn try_from_text(segment: BlankSegment<T, M>) -> Result<Self, Self::Error> {
        Ok(segment)
    }
}

impl<T, M> TryFromText<BlankText> for BlankSegment<T, M>
where
    T: RawText,
    M: MorphemeKind,
{
    type Error = MorphologyError;

    fn try_from_text(text: BlankText) -> Result<Self, Self::Error> {
        BlankSegment::try_from_width(text.0)
    }
}

// TODO: Segments ignore non-ASCII line breaks (by design). Make sure this is documented.
// TODO: Consider `unicode-linebreak` or something similar if it seems that support for line
//       breaking Unicode control characters like LS and PS is justified.
#[derive_where(Clone, Copy, Debug, Eq, Hash, PartialEq; T)]
#[repr(transparent)]
pub struct ContentSegment<T = String, M = FlexKind> {
    text: T,
    _phantom: PhantomData<fn() -> M>,
}

impl<T, M> ContentSegment<T, M> {
    fn from_raw_text_unchecked(text: T) -> Self {
        ContentSegment {
            text,
            _phantom: PhantomData,
        }
    }
}

impl<T, M> ContentSegment<T, M>
where
    T: RawText,
    M: MorphemeKind,
{
    pub fn try_from_raw_text<U>(text: U) -> Result<Self, MorphologyError>
    where
        T: RawText + TryFrom<MoveCow<U>>,
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
            Ok(ContentSegment::from_raw_text_unchecked(text))
        }
        else {
            Err(MorphologyError)
        }
    }

    pub fn try_from_raw_text_or_joined<U>(text: U) -> Result<Self, MorphologyError>
    where
        T: TryFrom<MoveCow<String>> + TryFrom<MoveCow<U>>,
        U: RawText,
    {
        let mut lines = text.as_ref().split_at_ascii_line_breaks().peekable();
        let first = lines.next();
        if lines.peek().is_some() {
            ContentSegment::try_from_text(first.into_iter().chain(lines).join(""))
        }
        else {
            drop(lines);
            ContentSegment::try_from_text(text)
        }
    }

    // TODO: Remove this. This is probably better suited to `Line`, not `Segment`.
    pub fn try_from_raw_text_or_split<U>(text: U) -> Result<Vec<Self>, MorphologyError>
    where
        // TODO: This requires that split text is copied into a `String`, but may not if
        //       `split_at_ascii_line_breaks` were implemented by `U` and returned `MoveCow`
        //       instead.
        T: TryFrom<MoveCow<String>>,
        U: RawText,
    {
        text.as_ref()
            .split_at_ascii_line_breaks()
            .map(String::from)
            .map(ContentSegment::try_from_text)
            .collect()
    }

    pub fn from_raw_text_or_empty<U>(text: U) -> Self
    where
        T: TryFrom<MoveCow<U>>,
        U: RawText,
    {
        match ContentSegment::try_from_text(text) {
            Ok(text) => text,
            _ => ContentSegment::empty(),
        }
    }

    pub const fn empty() -> Self {
        ContentSegment {
            text: T::EMPTY,
            _phantom: PhantomData,
        }
    }

    pub fn try_map_raw_text<U, F>(self, f: F) -> Result<ContentSegment<U, M>, MorphologyError>
    where
        U: RawText + TryFrom<MoveCow<U>>,
        F: FnOnce(T) -> U,
    {
        let ContentSegment { text, .. } = self;
        ContentSegment::try_from_text(f(text))
    }

    pub fn as_str(&self) -> &str {
        AsRef::<str>::as_ref(self)
    }

    pub fn is_blank(&self) -> bool {
        self.morphemes()
            .map(Indexed::into_text)
            .all(|morpheme| morpheme.is_blank())
    }

    pub fn is_empty(&self) -> bool {
        self.text.as_ref().is_empty()
    }
}

impl<'t, M> ContentSegment<Cow<'t, str>, M>
where
    M: MorphemeKind,
{
    pub fn into_owned(self) -> ContentSegment<Cow<'static, str>, M> {
        let ContentSegment { text, .. } = self;
        ContentSegment::from_raw_text_unchecked(text.into_owned().into())
    }
}

impl<T, M> ops::Append for ContentSegment<T, M>
where
    T: RawText + ToStringMut,
    M: MorphemeKind,
{
    fn append(mut self, rhs: Self) -> Self {
        self.text.to_string_mut().push_str(rhs.as_str());
        self
    }
}

impl<T, M> AsRef<str> for ContentSegment<T, M>
where
    T: AsRef<str>,
{
    fn as_ref(&self) -> &str {
        self.text.as_ref()
    }
}

impl<T, M> BlockText for ContentSegment<T, M>
where
    T: RawText,
    M: MorphemeKind,
{
    type RawText = T;
    type Morpheme<'t>
        = MorphemeFor<'t, M>
    where
        Self: 't;
    type Index = usize;

    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<Self::Index, Grapheme<'_>>> {
        self.text.as_ref().graphemes()
    }
}

impl<T, M> ops::Extend for ContentSegment<T, M>
where
    T: RawText + ToStringMut,
    M: MorphemeKind,
{
    fn extend<'t, I>(&'t mut self, morphemes: I) -> usize
    where
        I: IntoIterator<Item = <Self::BlockText as BlockText>::Morpheme<'t>>,
    {
        self.text
            .to_string_mut()
            .extend(morphemes.into_iter().map(Morpheme::into_string));
        self.width()
    }
}

impl<T, M> From<BlankSegment<T, M>> for ContentSegment<T, M>
where
    T: From<String> + RawText,
    M: MorphemeKind,
{
    fn from(segment: BlankSegment<T, M>) -> Self {
        ContentSegment::from_raw_text_unchecked(segment.to_string().into())
    }
}

impl<T, M> LinearGeometry for ContentSegment<T, M>
where
    T: RawText,
    M: MorphemeKind,
{
    fn width(&self) -> usize {
        self.text.as_ref().width()
    }
}

impl<T, M, S> Render<S> for ContentSegment<T, M>
where
    T: RawText,
    M: MorphemeKind,
    S: AnsiPrefix,
{
    fn fmt(&self, formatter: &mut Formatter, context: &mut RenderContext<S>) -> fmt::Result {
        context.fmt_with_ansi_fence(formatter, self.as_str())
    }
}

impl<T, M> ops::Truncate for ContentSegment<T, M>
where
    T: RawText + ToStringMut,
    M: MorphemeKind,
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
                    .checked_add(grapheme.text.width())
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

impl<'t, M> TryFrom<Cow<'t, str>> for ContentSegment<Cow<'t, str>, M>
where
    M: MorphemeKind,
{
    type Error = MorphologyError;

    fn try_from(text: Cow<'t, str>) -> Result<Self, Self::Error> {
        ContentSegment::try_from_text(text)
    }
}

impl<'t, M> TryFrom<&'t str> for ContentSegment<&'t str, M>
where
    M: MorphemeKind,
{
    type Error = MorphologyError;

    fn try_from(text: &'t str) -> Result<Self, Self::Error> {
        ContentSegment::try_from_text(text)
    }
}

impl<M> TryFrom<String> for ContentSegment<String, M>
where
    M: MorphemeKind,
{
    type Error = MorphologyError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        ContentSegment::try_from_text(text)
    }
}

impl<T, M> TryFromText<ContentSegment<T, M>> for ContentSegment<T, M> {
    type Error = Infallible;

    fn try_from_text(segment: ContentSegment<T, M>) -> Result<Self, Self::Error> {
        Ok(segment)
    }
}

impl<T, U, M> TryFromText<U> for ContentSegment<T, M>
where
    T: RawText + TryFrom<MoveCow<U>>,
    U: RawText,
    M: MorphemeKind,
{
    type Error = MorphologyError;

    fn try_from_text(text: U) -> Result<Self, Self::Error> {
        ContentSegment::try_from_raw_text(text)
    }
}
