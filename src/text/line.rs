use itertools::Itertools;
use std::borrow::Cow;
use std::convert::Infallible;
use std::fmt::{self, Debug, Formatter};

use crate::cow::MoveCow;
use crate::text::annotation::AnnotatedText;
use crate::text::geometry::LinearGeometry;
use crate::text::morphology::{Grapheme, MorphemeFor, MorphemeKind};
use crate::text::render::{DisplayProxy, DisplayStyle, Render, RenderContext};
use crate::text::segment::{BlankSegment, ContentSegment, Segment, SegmentComposition};
use crate::text::style::AnsiPrefix;
use crate::text::{
    BlankText, BlockText, BlockTextProjection, Indexed, MorphologyError, RawText, StrExt as _,
    ToStringMut, TryFromText,
};

pub trait LineComposition: BlockTextProjection<BlockText = Line<Self::Composed>> {
    type Composed: SegmentComposition<MorphemeKind = Self::MorphemeKind>;
    type MorphemeKind: MorphemeKind;
}

impl<T, U, M> LineComposition for T
where
    Line<U>: BlockText,
    U: SegmentComposition<MorphemeKind = M>,
    T: BlockTextProjection<BlockText = Line<U>>,
    M: MorphemeKind,
{
    type Composed = U;
    type MorphemeKind = M;
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LineIndex {
    pub segment: usize,
    pub byte: usize,
}

// TODO: Annotation type parameters are captured by `T` here. That is, `Annotated` is
//       abstracted such that `Line` has fewer type parameters and need not forward nor be aware of
//       annotation types. This will probably make it easier to support `Fill` types with "computed
//       text". However, this prevents ergonomic type inference: take care to make this easy to
//       use, at least in the common case (probably `String` text with `Flex` morphemes).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Line<T = Segment> {
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

    pub fn try_from_split_raw_text<R>(text: R) -> Result<Vec<Self>, T::Error>
    where
        T: TryFromText<String>,
        R: RawText,
    {
        // This is not implemented via `Segment::try_from_split_raw_text` to avoid an additional
        // allocation.
        text.as_ref()
            .split_at_ascii_line_breaks()
            .map(String::from)
            .map(|text| {
                T::try_from_text(text)
                    .map(|segment| vec![segment])
                    .map(Line::from)
            })
            .collect()
    }

    pub fn try_from_segments<I>(segments: I) -> Result<Self, T::Error>
    where
        T: TryFromText<I::Item>,
        I: IntoIterator,
    {
        segments
            .into_iter()
            .map(T::try_from_text)
            .collect::<Result<Vec<_>, _>>()
            .map(Line::from)
    }

    pub fn appended(mut front: Self, mut back: Self) -> Self {
        front.append(&mut back);
        front
    }

    pub fn append(&mut self, line: &mut Self) {
        self.segments.append(&mut line.segments);
    }

    pub fn push(&mut self, segment: impl Into<T>) {
        self.segments.push(segment.into());
    }

    pub fn has_segments(&self) -> bool {
        !self.segments.is_empty()
    }
}

impl<T> Line<T>
where
    T: DisplayStyle,
{
    pub fn display(&self) -> DisplayProxy<'_, Self, <Self as DisplayStyle>::Style> {
        DisplayProxy::from_text(self)
    }
}

impl<T> Line<T>
where
    T: SegmentComposition,
{
    pub fn from_raw_text_or_empty<R>(text: R) -> Self
    where
        T: From<T::BlockText>,
        T::RawText: TryFrom<MoveCow<R>>,
        R: RawText,
    {
        let segment = ContentSegment::from_raw_text_or_empty(text);
        if segment.is_empty() {
            Line::empty()
        }
        else {
            Line::from(vec![Segment::from(segment).into()])
        }
    }

    pub fn try_from_joined_raw_text<R>(text: R) -> Result<Self, MorphologyError>
    where
        T: From<T::BlockText>,
        T::RawText: TryFrom<MoveCow<String>> + TryFrom<MoveCow<R>>,
        R: RawText,
    {
        ContentSegment::try_from_joined_raw_text(text)
            .map(Segment::from)
            .map(|segment| Line::from(vec![segment.into()]))
    }

    pub fn project_and_append_segments(self) -> Line<T::BlockText>
    where
        T::Composed: From<String> + ToStringMut,
    {
        Line {
            segments: self
                .segments
                .into_iter()
                .map(BlockTextProjection::into_block_text)
                .reduce(Segment::appended)
                .map(|appended| vec![appended])
                .unwrap_or_else(Vec::new),
        }
    }

    pub fn get(&self, index: usize) -> Option<&T::BlockText> {
        self.segments
            .get(index)
            .map(BlockTextProjection::as_block_text)
    }

    pub fn segments(
        &self,
    ) -> impl '_ + Clone + DoubleEndedIterator + ExactSizeIterator + Iterator<Item = &'_ T::BlockText>
    {
        self.segments.iter().map(BlockTextProjection::as_block_text)
    }

    // Unlike `fmt` and `display`, this string is not terminated with a new line.
    pub fn to_string(&self) -> Cow<'_, str> {
        match self.segments.len() {
            0 => "".into(),
            1 => self.segments[0].as_block_text().to_string(),
            _ => self.segments().map(Segment::to_string).join("").into(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.segments().all(Segment::is_empty)
    }
}

impl<T, M> Line<T>
where
    T: SegmentComposition<MorphemeKind = M>,
    M: MorphemeKind,
{
    pub fn try_from_width(width: usize) -> Result<Self, M::Error>
    where
        T: From<T::BlockText>,
    {
        BlankSegment::try_from_width(width)
            .map(Segment::from)
            .map(|segment| vec![segment.into()])
            .map(Line::from)
    }
}

impl<'t, T, M> Line<T>
where
    T: SegmentComposition<Composed = Cow<'t, str>, MorphemeKind = M>,
    T::Mapped<Segment<Cow<'static, str>, M>>: BlockTextProjection,
    M: MorphemeKind,
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

impl<T, M, A> Line<AnnotatedText<Segment<T, M>, A>>
where
    T: From<String> + RawText + ToStringMut,
    M: MorphemeKind,
    A: PartialEq,
{
    pub fn coalesce(self) -> Self {
        Line {
            segments: self
                .segments
                .into_iter()
                .coalesce(|previous, next| {
                    if previous.annotation == next.annotation {
                        Ok(previous.map_text(move |text| Segment::appended(text, next.text)))
                    }
                    else {
                        Err((previous, next))
                    }
                })
                .collect(),
        }
    }
}

impl<T, M> BlockText for Line<T>
where
    T: SegmentComposition<MorphemeKind = M>,
    M: MorphemeKind,
{
    type RawText = T::RawText;
    type Morpheme<'t>
        = MorphemeFor<'t, M>
    where
        Self: 't;
    type Index = LineIndex;

    fn graphemes(&self) -> impl '_ + Iterator<Item = Indexed<Self::Index, Grapheme<'_>>> {
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

impl<T> Default for Line<T> {
    fn default() -> Self {
        Line {
            segments: Default::default(),
        }
    }
}

impl<T> DisplayStyle for Line<T>
where
    T: DisplayStyle,
{
    type Style = T::Style;
}

impl<T> Extend<T> for Line<T> {
    fn extend<I>(&mut self, segments: I)
    where
        I: IntoIterator<Item = T>,
    {
        self.segments.extend(segments);
    }
}

impl<T, M> From<BlankSegment<T, M>> for Line<Segment<T, M>> {
    fn from(segment: BlankSegment<T, M>) -> Self {
        Line {
            segments: vec![segment.into()],
        }
    }
}

impl<T, M> From<ContentSegment<T, M>> for Line<Segment<T, M>> {
    fn from(segment: ContentSegment<T, M>) -> Self {
        Line {
            segments: vec![segment.into()],
        }
    }
}

impl<T, M> From<Segment<T, M>> for Line<Segment<T, M>> {
    fn from(segment: Segment<T, M>) -> Self {
        Line {
            segments: vec![segment],
        }
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

impl<T, M> LinearGeometry for Line<T>
where
    T: SegmentComposition<MorphemeKind = M>,
    M: MorphemeKind,
{
    fn width(&self) -> usize {
        self.segments
            .iter()
            .map(BlockTextProjection::as_block_text)
            .map(LinearGeometry::width)
            .sum()
    }
}

impl<T, S> Render<S> for Line<T>
where
    T: Render<S>,
    S: AnsiPrefix,
{
    fn fmt(&self, formatter: &mut Formatter, context: &mut RenderContext<S>) -> fmt::Result {
        for segment in &self.segments {
            segment.fmt(formatter, context)?;
        }
        writeln!(formatter)?;
        Ok(())
    }
}

impl<T, X, A> TryFromText<AnnotatedText<X, A>> for Line<AnnotatedText<T, A>>
where
    T: TryFromText<X>,
{
    type Error = T::Error;

    fn try_from_text(annotated: AnnotatedText<X, A>) -> Result<Self, Self::Error> {
        annotated
            .map_text(T::try_from_text)
            .transpose()
            .map(|segment| Line::from(vec![segment]))
    }
}

impl<T> TryFromText<BlankText> for Line<T>
where
    T: TryFromText<BlankText>,
{
    type Error = T::Error;

    fn try_from_text(text: BlankText) -> Result<Self, Self::Error> {
        T::try_from_text(text).map(|segment| Line::from(vec![segment]))
    }
}

impl<T> TryFromText<Line<T>> for Line<T> {
    type Error = Infallible;

    fn try_from_text(line: Line<T>) -> Result<Self, Self::Error> {
        Ok(line)
    }
}

impl<T, X> TryFromText<X> for Line<T>
where
    T: TryFromText<X>,
    X: RawText,
{
    type Error = T::Error;

    fn try_from_text(text: X) -> Result<Self, Self::Error> {
        T::try_from_text(text).map(|segment| Line::from(vec![segment]))
    }
}
