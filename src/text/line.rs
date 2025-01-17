use itertools::Itertools;
use std::borrow::Cow;
use std::convert::Infallible;
use std::fmt::{self, Debug, Display, Formatter};

use crate::cow::MoveCow;
use crate::slice::{SliceExt as _, SliceProjection};
use crate::text::annotation::AnnotatedText;
use crate::text::geometry::{AsBlockGeometry, BlockGeometry, LinearGeometry};
use crate::text::morphology::{Grapheme, MorphemeFor, MorphemeKind};
use crate::text::render::{AsDisplay, Render, RenderContext};
use crate::text::segment::{BlankSegment, ContentSegment, Segment, SegmentFor};
use crate::text::style::AnsiPrefix;
use crate::text::{
    ops, BlankText, BlockText, BlockTextProjection, Indexed, MorphologyError, RawText, StrExt as _,
    TryFromText,
};

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
    //       necessary in code written against `Line`, consider making this change. The same idea
    //       probably applies to `Block` too: `Line`s could be pushed onto the "top" or "bottom" of
    //       a block.
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
    T: BlockTextProjection<BlockText = Segment<<T as BlockTextProjection>::RawText, M>>,
    M: MorphemeKind,
{
    pub fn try_from_raw_text_or_joined<U>(text: U) -> Result<Self, MorphologyError>
    where
        T: From<Segment<T::RawText, M>>,
        T::RawText: TryFrom<MoveCow<String>> + TryFrom<MoveCow<U>>,
        U: RawText,
    {
        ContentSegment::try_from_raw_text_or_joined(text)
            .map(Segment::from)
            .map(|segment| Line::from(vec![segment.into()]))
    }

    pub fn try_from_raw_text_or_split<U>(text: U) -> Result<Vec<Self>, T::Error>
    where
        T: TryFromText<String>,
        U: RawText,
    {
        // This is not implemented via `Segment::try_from_raw_text_or_split` to avoid an additional
        // allocation.
        text.as_ref()
            .split_at_ascii_line_breaks()
            .map(String::from)
            .map(Line::try_from_text)
            .collect()
    }

    pub fn try_from_width(width: usize) -> Result<Self, MorphologyError>
    where
        MorphologyError: From<T::Error>,
        T: TryFromText<Segment<T::RawText, M>>,
    {
        BlankSegment::try_from_width(width)
            .map(Segment::from)
            .and_then(|segment| T::try_from_text(segment).map_err(Into::into))
            .map(|segment| Line::from(vec![segment]))
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

    pub fn push(&mut self, segment: impl Into<T>) {
        self.segments.push(segment.into());
    }

    pub fn concatenate(self) -> Line<Segment<<T as BlockTextProjection>::RawText, M>>
    where
        Segment<<T as BlockTextProjection>::RawText, M>: ops::Append,
    {
        Line {
            segments: self
                .segments
                .into_iter()
                .map(BlockTextProjection::into_block_text)
                .reduce(ops::Append::append)
                .map(|concatenated| vec![concatenated])
                .unwrap_or_else(Vec::new),
        }
    }

    pub fn get(&self, index: usize) -> Option<&SegmentFor<T, M>> {
        self.segments
            .get(index)
            .map(BlockTextProjection::as_block_text)
    }

    pub fn segments(&self) -> impl '_ + SliceProjection<Item = SegmentFor<T, M>> {
        self.segments
            .as_slice()
            .project(BlockTextProjection::as_block_text)
    }

    // Unlike `fmt` and `display`, this string is not terminated with a new line.
    pub fn to_string(&self) -> Cow<'_, str> {
        let segments = self.segments();
        match segments.len() {
            0 => "".into(),
            // TODO: Why can this not be done through the slice projection...? Fix this, if
            //       possible.
            //1 => segments.get(0).unwrap().to_string(),
            1 => self.segments[0].as_block_text().to_string(),
            _ => segments.iter().map(Segment::to_string).join("").into(),
        }
    }

    pub fn display<'d, S>(&'d self) -> impl 'd + Display
    where
        Self: Render<S>,
        S: 'd + AnsiPrefix,
    {
        AsDisplay::from(self)
    }

    // CLIPPY: This appears to be a false positive. An explicit lifetime is necessary for the GATs.
    #[allow(clippy::needless_lifetimes)]
    pub fn as_block_geometry<'b>(
        &'b self,
    ) -> impl 'b + BlockGeometry<Morpheme<'b> = MorphemeFor<'b, M>, Index = LineIndex> {
        AsBlockGeometry(self)
    }

    pub fn has_segments(&self) -> bool {
        !self.segments.is_empty()
    }

    pub fn is_empty(&self) -> bool {
        self.segments().iter().all(|segment| segment.is_empty())
    }
}

// TODO: `into_owned` functions like these cannot detect and clone annotations. Abstract this
//       further with an `IntoOwned` trait. Types like
//       `Annotation<Segment<Cow<'_, str>, _>, Styler<&'_ Style>>` can implement this trait
//       transitively over its fields, allowing both the text and style to clone.
impl<'t, T, M> Line<T>
where
    T: BlockTextProjection<BlockText = Segment<Cow<'t, str>, M>>,
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

impl<T, A> Line<AnnotatedText<T, A>>
where
    T: ops::Append,
    A: Eq,
{
    pub fn coalesce(self) -> Self {
        Line {
            segments: self
                .segments
                .into_iter()
                .coalesce(|previous, next| {
                    if previous.annotation == next.annotation {
                        Ok(previous.map_text(move |text| ops::Append::append(text, next.text)))
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
    T: BlockTextProjection<BlockText = Segment<<T as BlockTextProjection>::RawText, M>>,
    M: MorphemeKind,
{
    type RawText = T::RawText;
    type Morpheme<'t>
        = MorphemeFor<'t, M>
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

impl<T> Default for Line<T> {
    fn default() -> Self {
        Line {
            segments: Default::default(),
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
    T: BlockTextProjection<BlockText = Segment<<T as BlockTextProjection>::RawText, M>>,
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

// TODO: It may be a good idea to `coalesce` styled lines to avoid unnecessary ANSI escape codes.
//       This can't be done in the `Render` trait without a clone though.
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

impl<T, M, U, A> TryFromText<AnnotatedText<U, A>> for Line<AnnotatedText<T, A>>
where
    T: BlockTextProjection<BlockText = Segment<<T as BlockTextProjection>::RawText, M>>
        + TryFromText<U>,
    M: MorphemeKind,
{
    type Error = T::Error;

    fn try_from_text(annotated: AnnotatedText<U, A>) -> Result<Self, Self::Error> {
        annotated
            .map_text(T::try_from_text)
            .transpose()
            .map(|segment| Line::from(vec![segment]))
    }
}

impl<T, M> TryFromText<BlankText> for Line<T>
where
    T: BlockTextProjection<BlockText = Segment<<T as BlockTextProjection>::RawText, M>>
        + TryFromText<BlankText>,
    M: MorphemeKind,
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

impl<T, M, U> TryFromText<U> for Line<T>
where
    T: BlockTextProjection<BlockText = Segment<<T as BlockTextProjection>::RawText, M>>
        + TryFromText<U>,
    M: MorphemeKind,
    U: RawText,
{
    type Error = T::Error;

    fn try_from_text(text: U) -> Result<Self, Self::Error> {
        T::try_from_text(text).map(|segment| Line::from(vec![segment]))
    }
}
