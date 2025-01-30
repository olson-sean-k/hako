use derive_where::derive_where;
use std::borrow::Cow;
use std::convert::Infallible;
use std::fmt::{self, Debug, Formatter};
use std::iter;
use std::marker::PhantomData;

use crate::text::annotation::AnnotatedText;
use crate::text::geometry::{BlockGeometry, BoundingBox, LinearGeometry};
use crate::text::line::{Line, LineComposition, LineIndex};
use crate::text::modal::ModalText;
use crate::text::morphology::{Grapheme, MorphemeFor, MorphemeKind};
use crate::text::render::{
    DisplayProxy, DisplayStyle, Render, RenderContext, RenderFn, RenderNode,
};
use crate::text::segment::{BlankSegment, Segment, SegmentComposition};
use crate::text::style::AnsiPrefix;
use crate::text::{
    BlankText, BlockText, BlockTextProjection, Indexed, IteratorExt as _, RawText, StrExt as _,
    TryFromText,
};

use ModalText::{Blank, Content};

type ModalBlock<T> = ModalText<BlankBlock<T>, ContentBlock<T>>;

pub trait BlockComposition: BlockTextProjection<BlockText = Block<Self::Composed>> {
    type Composed: LineComposition<MorphemeKind = Self::MorphemeKind>;
    type MorphemeKind: MorphemeKind;
}

impl<T, U, M> BlockComposition for T
where
    Block<U>: BlockText,
    T: BlockTextProjection<BlockText = Block<U>>,
    U: LineComposition<MorphemeKind = M>,
    M: MorphemeKind,
{
    type Composed = U;
    type MorphemeKind = M;
}

// TODO: Index types can only be used with their corresponding block text type, because some
//       outputs are "synthesized" and don't refer to an existing index in composed block text.
//       These type should probably be entirely opaque (i.e., no exported fields nor accessors).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BlockIndex {
    pub line: usize,
    pub segment: usize,
    pub byte: usize,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Block<T = Line> {
    modal: ModalBlock<T>,
}

impl<T> Block<T>
where
    T: DisplayStyle,
{
    pub fn display(&self) -> DisplayProxy<'_, Self, <Self as DisplayStyle>::Style> {
        DisplayProxy::from_text(self)
    }
}

impl<T, M> BlockText for Block<T>
where
    T: LineComposition<MorphemeKind = M>,
    M: MorphemeKind,
{
    type RawText = T::RawText;
    type Morpheme<'t>
        = MorphemeFor<'t, M>
    where
        Self: 't;
    type Index = BlockIndex;

    fn morphemes(&self) -> impl '_ + Iterator<Item = Indexed<Self::Index, Self::Morpheme<'_>>> {
        self.modal
            .as_ref()
            .map_blank(BlankBlock::morphemes)
            .map_content(ContentBlock::morphemes)
    }

    fn graphemes(&self) -> impl '_ + Iterator<Item = Indexed<Self::Index, Grapheme<'_>>> {
        self.modal
            .as_ref()
            .map_blank(BlankBlock::graphemes)
            .map_content(ContentBlock::graphemes)
    }
}

impl<T> DisplayStyle for Block<T>
where
    T: DisplayStyle,
{
    type Style = T::Style;
}

impl<T> From<BlankBlock<T>> for Block<T> {
    fn from(block: BlankBlock<T>) -> Self {
        Block {
            modal: Blank(block),
        }
    }
}

impl<T> From<ContentBlock<T>> for Block<T> {
    fn from(block: ContentBlock<T>) -> Self {
        Block {
            modal: Content(block),
        }
    }
}

impl<T, M, S> Render<S> for Block<T>
where
    T: LineComposition<MorphemeKind = M> + Render<S>,
    M: MorphemeKind,
    M::Error: Debug,
    S: AnsiPrefix,
{
    fn fmt(&self, formatter: &mut Formatter, context: &mut RenderContext<S>) -> fmt::Result {
        match self.modal {
            Blank(ref blank) => Render::fmt(blank, formatter, context),
            Content(ref content) => Render::fmt(content, formatter, context),
        }
    }
}

#[derive_where(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BlankBlock<T = Line> {
    width: usize,
    height: usize,
    _phantom: PhantomData<fn() -> T>,
}

impl<T> BlankBlock<T> {
    pub const fn empty() -> Self {
        BlankBlock::from_dimensions(0, 0)
    }

    pub const fn from_dimensions(width: usize, height: usize) -> Self {
        BlankBlock {
            width,
            height,
            _phantom: PhantomData,
        }
    }

    pub const fn has_lines(&self) -> bool {
        self.height != 0
    }
}

impl<T, M> BlockGeometry for BlankBlock<T>
where
    T: LineComposition<MorphemeKind = M>,
    M: MorphemeKind,
{
    type Height = usize;

    fn ascii_line_break_bounds(&self) -> BoundingBox<Self::Height> {
        BoundingBox {
            width: self.width,
            height: self.height,
        }
    }
}

impl<T, M> BlockText for BlankBlock<T>
where
    T: LineComposition<MorphemeKind = M>,
    M: MorphemeKind,
{
    type RawText = T::RawText;
    type Morpheme<'t>
        = MorphemeFor<'t, M>
    where
        Self: 't;
    type Index = BlockIndex;

    fn graphemes(&self) -> impl '_ + Iterator<Item = Indexed<Self::Index, Grapheme<'_>>> {
        self.morphemes()
            .map(|morpheme| morpheme.map_text(Into::into))
    }

    fn morphemes(&self) -> impl '_ + Iterator<Item = Indexed<Self::Index, Self::Morpheme<'_>>> {
        iter::repeat(M::blanks_in_width(self.width))
            .enumerate()
            .take(self.height)
            .flat_map(move |(index, line)| {
                line.enumerate().map(
                    move |(
                        byte,
                        Indexed {
                            index: segment,
                            text,
                        },
                    )| Indexed {
                        index: BlockIndex {
                            line: index,
                            segment,
                            byte,
                        },
                        text,
                    },
                )
            })
    }
}

impl<T, M, S> Render<S> for BlankBlock<T>
where
    T: LineComposition<MorphemeKind = M> + Render<S>,
    M: MorphemeKind,
    M::Error: Debug,
    S: AnsiPrefix,
{
    fn fmt(&self, formatter: &mut Formatter, context: &mut RenderContext<S>) -> fmt::Result {
        let line = Line::from(BlankSegment::<T::RawText, M>::from_width_unchecked(
            self.width,
        ));
        for _ in 0..self.height {
            Render::<S>::fmt(&line, formatter, context)?;
        }
        Ok(())
    }
}

// TODO: There's a fundamental question here about line padding: should blank segments be pushed to
//       pad lines, or should `ContentBlock` do "in-place" padding when implementating traits like
//       `BlockText`?
// TODO: Enforce padding, which is very easy to omit by mistake, with type state. This may require
//       replacing `&mut self` receivers with `self`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ContentBlock<T = Line> {
    lines: Vec<T>,
}

impl<T> ContentBlock<T> {
    pub const fn empty() -> Self {
        ContentBlock { lines: Vec::new() }
    }

    pub fn push(&mut self, line: impl Into<T>) {
        self.lines.push(line.into());
    }

    pub fn has_lines(&self) -> bool {
        !self.lines.is_empty()
    }
}

impl<T> ContentBlock<T>
where
    T: LineComposition,
{
    pub fn try_from_lines<I>(lines: I) -> Result<Self, T::Error>
    where
        T: TryFromText<I::Item>,
        I: IntoIterator,
    {
        lines
            .into_iter()
            .map(T::try_from_text)
            .collect::<Result<Vec<_>, _>>()
            .map(ContentBlock::from)
    }

    pub fn try_from_split_raw_text<R>(
        text: R,
    ) -> Result<Self, <T::BlockText as TryFromText<String>>::Error>
    where
        T: From<T::BlockText>,
        T::BlockText: TryFromText<String>,
        R: RawText,
    {
        // This is not implemented via `Line::try_from_split_raw_text` to avoid an additional
        // allocation.
        text.as_ref()
            .split_at_ascii_line_breaks()
            .map(String::from)
            .map(|text| T::BlockText::try_from_text(text).map(T::from))
            .collect::<Result<Vec<_>, _>>()
            .map(ContentBlock::from)
    }

    pub fn get(&self, index: usize) -> Option<&Line<T::Composed>> {
        self.lines
            .get(index)
            .map(BlockTextProjection::as_block_text)
    }

    pub fn lines(
        &self,
    ) -> impl '_ + Clone + DoubleEndedIterator + ExactSizeIterator + Iterator<Item = &'_ Line<T::Composed>>
    {
        self.lines.iter().map(BlockTextProjection::as_block_text)
    }

    pub fn is_empty(&self) -> bool {
        self.lines().all(Line::is_empty)
    }
}

impl<'t, T, U, M> ContentBlock<T>
where
    T: LineComposition<Composed = U, MorphemeKind = M>,
    T::Mapped<Line<U::Mapped<Segment<Cow<'static, str>, M>>>>: BlockTextProjection,
    U: SegmentComposition<Composed = Cow<'t, str>, MorphemeKind = M>,
    U::Mapped<Segment<Cow<'static, str>, M>>: BlockTextProjection,
    M: MorphemeKind,
{
    pub fn into_owned(
        self,
    ) -> ContentBlock<T::Mapped<Line<U::Mapped<Segment<Cow<'static, str>, M>>>>> {
        let ContentBlock { lines } = self;
        ContentBlock {
            lines: lines
                .into_iter()
                .map(|line| line.map_block_text(Line::into_owned))
                .collect(),
        }
    }
}

impl<T, M> BlockGeometry for ContentBlock<T>
where
    T: LineComposition<MorphemeKind = M>,
    M: MorphemeKind,
{
    type Height = usize;

    fn ascii_line_break_bounds(&self) -> BoundingBox<Self::Height> {
        BoundingBox {
            width: self.lines().map(Line::width).max().unwrap_or(0),
            height: self.lines.len(),
        }
    }
}

impl<T, M> BlockText for ContentBlock<T>
where
    T: LineComposition<MorphemeKind = M>,
    M: MorphemeKind,
{
    type RawText = T::RawText;
    type Morpheme<'t>
        = MorphemeFor<'t, M>
    where
        Self: 't;
    type Index = BlockIndex;

    fn graphemes(&self) -> impl '_ + Iterator<Item = Indexed<Self::Index, Grapheme<'_>>> {
        let width = self.ascii_line_break_bounds().width;
        self.lines()
            .margins(width)
            .enumerate()
            .flat_map(|(index, (margin, line))| {
                // An empty segment may need to be synthesized to pad the line. Stash its index,
                // which is the length of the buffer of segments in the line. See below.
                let segment = line.segments().len();
                line.graphemes()
                    .map(move |grapheme| {
                        grapheme.map_index(|LineIndex { segment, byte }| BlockIndex {
                            line: index,
                            segment,
                            byte,
                        })
                    })
                    .chain(
                        // TODO: Unlike the `Render` implementation, which constructs a
                        //       `BlankSegment`, this does not assert that morphemes can represent
                        //       this width exactly. These implementations should probably do the
                        //       same thing in this regard (trust that the margin is correct or
                        //       check it and panic).
                        // Pad the line to the width of the block.
                        M::blanks_in_width(margin)
                            .map(|morpheme| morpheme.map_text(Into::into))
                            .map(move |grapheme| {
                                grapheme.map_index(|byte| BlockIndex {
                                    line: index,
                                    segment, // Synthesized segment index. See above.
                                    byte,
                                })
                            }),
                    )
            })
    }
}

impl<T> Default for ContentBlock<T> {
    fn default() -> Self {
        ContentBlock {
            lines: Default::default(),
        }
    }
}

impl<T> DisplayStyle for ContentBlock<T>
where
    T: DisplayStyle,
{
    type Style = T::Style;
}

impl<T> Extend<T> for ContentBlock<T> {
    fn extend<I>(&mut self, lines: I)
    where
        I: IntoIterator<Item = T>,
    {
        self.lines.extend(lines);
    }
}

impl<T> From<Line<T>> for ContentBlock<Line<T>> {
    fn from(line: Line<T>) -> Self {
        ContentBlock::from_iter([line])
    }
}

impl<T> From<Vec<T>> for ContentBlock<T> {
    fn from(lines: Vec<T>) -> Self {
        ContentBlock { lines }
    }
}

impl<T> FromIterator<T> for ContentBlock<T>
where
    Self: From<Vec<T>>,
{
    fn from_iter<I>(lines: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        ContentBlock::from(lines.into_iter().collect::<Vec<_>>())
    }
}

impl<T, M, S> Render<S> for ContentBlock<T>
where
    T: LineComposition<MorphemeKind = M> + Render<S>,
    M: MorphemeKind,
    S: AnsiPrefix,
{
    fn fmt(&self, formatter: &mut Formatter, context: &mut RenderContext<S>) -> fmt::Result {
        let width = self.ascii_line_break_bounds().width;
        context.push_node_and_fmt(formatter, || {
            (
                RenderNode::BlockWidth(width),
                RenderFn::from(|formatter, context| {
                    for line in &self.lines {
                        line.fmt(formatter, context)?;
                    }
                    Ok(())
                }),
            )
        })
    }
}

impl<T, U, A> TryFromText<AnnotatedText<U, A>> for ContentBlock<AnnotatedText<T, A>>
where
    Self: From<Vec<AnnotatedText<T, A>>>,
    T: TryFromText<U>,
{
    type Error = T::Error;

    fn try_from_text(annotated: AnnotatedText<U, A>) -> Result<Self, Self::Error> {
        annotated
            .map_text(T::try_from_text)
            .transpose()
            .map(|line| ContentBlock::from(vec![line]))
    }
}

impl<T> TryFromText<BlankText> for ContentBlock<T>
where
    Self: From<Vec<T>>,
    T: TryFromText<BlankText>,
{
    type Error = T::Error;

    fn try_from_text(text: BlankText) -> Result<Self, Self::Error> {
        T::try_from_text(text).map(|line| ContentBlock::from(vec![line]))
    }
}

impl<T> TryFromText<ContentBlock<T>> for ContentBlock<T> {
    type Error = Infallible;

    fn try_from_text(line: ContentBlock<T>) -> Result<Self, Self::Error> {
        Ok(line)
    }
}

impl<T, R> TryFromText<R> for ContentBlock<T>
where
    Self: From<Vec<T>>,
    T: TryFromText<R>,
    R: RawText,
{
    type Error = T::Error;

    fn try_from_text(text: R) -> Result<Self, Self::Error> {
        T::try_from_text(text).map(|line| ContentBlock::from(vec![line]))
    }
}
