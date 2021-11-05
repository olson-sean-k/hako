use std::borrow::Cow;
use std::cmp;
use std::marker::PhantomData;
use std::ops::Deref;

use crate::align::{
    Axis, Bottom, HorizontalAlignment, Left, LeftRight, Right, Top, TopBottom, VerticalAlignment,
};
use crate::content::{Congruent, Content, ContentSlice as _, Grapheme, Layer, Style, Styled};
use crate::Render;

pub trait Fill<C, T>
where
    C: Content,
{
    type Output;

    fn fill(self, filler: T) -> Self::Output;
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct EmptyBlock {
    width: usize,
    height: usize,
}

impl EmptyBlock {
    pub fn new(width: usize, height: usize) -> Self {
        EmptyBlock { width, height }
    }
}

/// Fundamental operations.
impl EmptyBlock {
    pub fn pad_to_width_at_right(self, width: usize) -> Self {
        EmptyBlock {
            width: cmp::max(self.width, width),
            height: self.height,
        }
    }

    pub fn pad_to_height_at_bottom(self, height: usize) -> Self {
        EmptyBlock {
            width: self.width,
            height: cmp::max(self.height, height),
        }
    }

    pub fn join_left_to_right_at_top(self, right: Self) -> Self {
        let left = self;
        EmptyBlock {
            width: left.width + right.width,
            height: cmp::max(left.height, right.height),
        }
    }

    pub fn join_top_to_bottom_at_left(self, bottom: Self) -> Self {
        let top = self;
        EmptyBlock {
            width: cmp::max(top.width, bottom.width),
            height: top.height + bottom.height,
        }
    }

    pub fn overlay(self, back: Self) -> Self {
        let front = self;
        EmptyBlock {
            width: cmp::max(front.width, back.width),
            height: cmp::max(front.height, back.height),
        }
    }
}

impl<'t, C> Fill<C, C> for EmptyBlock
where
    C: Content,
{
    type Output = Result<ContentBlock<C>, Self>;

    fn fill(self, content: C) -> Self::Output {
        fn div_ceiling(a: usize, b: usize) -> usize {
            (0..a).step_by(b).len()
        }

        if self.height == 0 {
            Err(self)
        }
        else {
            let mut lines = content.into_lines();
            let n = lines.len();
            if n < self.height {
                let mut i = 0usize;
                for _ in 0..(self.height - n) {
                    let line = lines.get(i).unwrap().clone();
                    lines.push(line);
                    i = (i + 1) % n;
                }
            }
            while lines.len() > self.height {
                lines.pop();
            }
            for line in lines.iter_mut() {
                if line.width() < self.width {
                    let n = div_ceiling(self.width, line.width());
                    *line = line.clone().repeat(n);
                }
                *line = line.clone().truncate(self.width);
            }
            Ok(lines.into())
        }
    }
}

impl<'t, C> Fill<C, Grapheme<'t>> for EmptyBlock
where
    C: Content,
{
    type Output = Result<ContentBlock<C>, Self>;

    fn fill(self, glyph: Grapheme<'t>) -> Self::Output {
        if self.height == 0 {
            Err(self)
        }
        else {
            Ok(ContentBlock {
                lines: vec![C::grapheme(glyph).repeat(self.width); self.height],
            })
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ContentBlock<C>
where
    C: Content,
{
    lines: Vec<C>,
}

impl<C> ContentBlock<C>
where
    C: Content,
{
    pub fn push(self, content: impl Into<C>) -> Self {
        let ContentBlock { mut lines } = self;
        lines.extend(content.into().into_lines());
        ContentBlock::from(lines)
    }

    pub fn width(&self) -> usize {
        self.lines.width()
    }

    pub fn height(&self) -> usize {
        self.lines.len()
    }

    fn normalize(self, width: usize) -> Self {
        ContentBlock {
            lines: self
                .lines
                .into_iter()
                .map(|line| {
                    let n = width.saturating_sub(line.width());
                    if n > 0 {
                        Content::concatenate(line, C::grapheme(Grapheme::SPACE).repeat(n))
                    }
                    else {
                        line
                    }
                })
                .collect(),
        }
    }
}

/// Fundamental operations.
impl<C> ContentBlock<C>
where
    C: Content,
{
    pub fn pad_to_width_at_right(self, width: usize) -> Self {
        let width = width.saturating_sub(self.width());
        if width > 0 {
            ContentBlock {
                lines: self
                    .lines
                    .into_iter()
                    .map(|line| {
                        // This assumes that lines are properly padded such that
                        // they have equal width (and so no per-line width must
                        // be computed).
                        Content::concatenate(line, C::space().repeat(width))
                    })
                    .collect(),
            }
        }
        else {
            self
        }
    }

    pub fn pad_to_height_at_bottom(self, height: usize) -> Self {
        let height = height.saturating_sub(self.height());
        if height > 0 {
            // Because `height` is greater than zero, the fill cannot fail.
            let padding = EmptyBlock::new(self.width(), height)
                .fill(Grapheme::SPACE)
                .unwrap();
            self.join_top_to_bottom_at_left(padding)
        }
        else {
            self
        }
    }

    pub fn join_left_to_right_at_top(self, right: Self) -> Self {
        let height = cmp::max(self.height(), right.height());
        let left = self.pad_to_height_at_bottom(height);
        let right = right.pad_to_height_at_bottom(height);
        ContentBlock {
            lines: left
                .lines
                .into_iter()
                .zip(right.lines)
                .map(|(left, right)| C::concatenate(left, right))
                .collect(),
        }
    }

    pub fn join_top_to_bottom_at_left(self, bottom: Self) -> Self {
        let width = cmp::max(self.width(), bottom.width());
        let top = self.pad_to_width_at_right(width);
        let bottom = bottom.pad_to_width_at_right(width);
        ContentBlock {
            lines: top.lines.into_iter().chain(bottom.lines).collect(),
        }
    }

    pub fn overlay(self, back: Self) -> Self {
        let width = cmp::max(self.width(), back.width());
        let height = cmp::max(self.height(), back.height());
        let front = self
            .pad_to_height_at_bottom(height)
            .pad_to_width_at_right(width);
        let back = back
            .pad_to_height_at_bottom(height)
            .pad_to_width_at_right(width);
        let lines: Vec<_> = front
            .lines
            .into_iter()
            .zip(back.lines)
            .map(|(front, back)| {
                Content::overlay_zip_with(
                    Congruent::try_from((front, back)).unwrap(),
                    |front, _| {
                        if front == Grapheme::SPACE {
                            Layer::Back(())
                        }
                        else {
                            Layer::Front(())
                        }
                    },
                )
            })
            .collect();
        lines.into()
    }
}

impl<'t> ContentBlock<Cow<'t, str>> {
    pub fn into_owned(self) -> ContentBlock<Cow<'static, str>> {
        ContentBlock {
            lines: self
                .lines
                .into_iter()
                .map(|line| line.into_owned().into())
                .collect(),
        }
    }
}

impl<C, S> ContentBlock<Styled<C, S>>
where
    C: AsRef<str> + Content + From<String>,
    S: Default + Style,
{
    pub fn restyle(self, style: S) -> Self {
        ContentBlock {
            lines: self
                .lines
                .into_iter()
                .map(|line| line.restyle(style.clone()))
                .collect(),
        }
    }
}

impl<C> From<Vec<C>> for ContentBlock<C>
where
    C: Content,
{
    fn from(lines: Vec<C>) -> Self {
        let width = lines.width();
        ContentBlock { lines }.normalize(width)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum ModalBlock<C>
where
    C: Content,
{
    Empty(EmptyBlock),
    Content(ContentBlock<C>),
}

impl<C> ModalBlock<C>
where
    C: Content,
{
    pub fn width(&self) -> usize {
        match self {
            ModalBlock::Empty(ref block) => block.width,
            ModalBlock::Content(ref block) => block.width(),
        }
    }

    pub fn height(&self) -> usize {
        match self {
            ModalBlock::Empty(ref block) => block.height,
            ModalBlock::Content(ref block) => block.height(),
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, ModalBlock::Empty(_))
    }
}

// TODO: Generalize and share code for joins and overlays.
// TODO: Only pre-pad the heights of empty blocks to allow filling.
/// Fundamental operations.
impl<C> ModalBlock<C>
where
    C: Content,
{
    pub fn pad_to_width_at_right(self, width: usize) -> Self {
        match self {
            ModalBlock::Empty(block) => block.pad_to_width_at_right(width).into(),
            ModalBlock::Content(block) => block.pad_to_width_at_right(width).into(),
        }
    }

    pub fn pad_to_height_at_bottom(self, height: usize) -> Self {
        match self {
            ModalBlock::Empty(block) => block.pad_to_height_at_bottom(height).into(),
            ModalBlock::Content(block) => block.pad_to_height_at_bottom(height).into(),
        }
    }

    pub fn join_left_to_right_at_top(self, right: Self) -> Self {
        match (self, right) {
            (ModalBlock::Empty(left), ModalBlock::Empty(right)) => {
                left.join_left_to_right_at_top(right).into()
            }
            (ModalBlock::Content(left), ModalBlock::Content(right)) => {
                left.join_left_to_right_at_top(right).into()
            }
            (ModalBlock::Empty(left), ModalBlock::Content(right)) => {
                if left.width == 0 {
                    right
                }
                else {
                    // Pad eagerly to expand the height of the empty block
                    // beyond zero.
                    let height = cmp::max(left.height, right.height());
                    let left = left.pad_to_height_at_bottom(height);
                    let right = right.pad_to_height_at_bottom(height);
                    // Neither the width nor height of the empty block can be
                    // zero here, so the fill cannot fail.
                    left.fill(Grapheme::SPACE)
                        .unwrap()
                        .join_left_to_right_at_top(right)
                }
                .into()
            }
            (ModalBlock::Content(left), ModalBlock::Empty(right)) => {
                if right.width == 0 {
                    left
                }
                else {
                    // Pad eagerly to expand the height of the empty block
                    // beyond zero.
                    let height = cmp::max(left.height(), right.height);
                    let left = left.pad_to_height_at_bottom(height);
                    let right = right.pad_to_height_at_bottom(height);
                    // Neither the width nor height of the empty block can be
                    // zero here, so the fill cannot fail.
                    left.join_left_to_right_at_top(right.fill(Grapheme::SPACE).unwrap())
                }
                .into()
            }
        }
    }

    pub fn join_top_to_bottom_at_left(self, bottom: Self) -> Self {
        match (self, bottom) {
            (ModalBlock::Empty(top), ModalBlock::Empty(bottom)) => {
                top.join_top_to_bottom_at_left(bottom).into()
            }
            (ModalBlock::Content(top), ModalBlock::Content(bottom)) => {
                top.join_top_to_bottom_at_left(bottom).into()
            }
            (ModalBlock::Empty(top), ModalBlock::Content(bottom)) => {
                if top.height == 0 {
                    bottom
                }
                else {
                    // Pad eagerly to expand the width of the empty block beyond
                    // zero.
                    let width = cmp::max(top.width, bottom.width());
                    let top = top.pad_to_width_at_right(width);
                    let bottom = bottom.pad_to_width_at_right(width);
                    // Neither the width nor height of the empty block can be
                    // zero here, so the fill cannot fail.
                    top.fill(Grapheme::SPACE)
                        .unwrap()
                        .join_top_to_bottom_at_left(bottom)
                }
                .into()
            }
            (ModalBlock::Content(top), ModalBlock::Empty(bottom)) => {
                if bottom.height == 0 {
                    top
                }
                else {
                    // Pad eagerly to expand the width of the empty block beyond
                    // zero.
                    let width = cmp::max(top.width(), bottom.width);
                    let top = top.pad_to_width_at_right(width);
                    let bottom = bottom.pad_to_width_at_right(width);
                    // Neither the width nor height of the empty block can be
                    // zero here, so the fill cannot fail.
                    top.join_top_to_bottom_at_left(bottom.fill(Grapheme::SPACE).unwrap())
                }
                .into()
            }
        }
    }

    pub fn overlay(self, back: Self) -> Self {
        match (self, back) {
            (ModalBlock::Empty(front), ModalBlock::Empty(back)) => front.overlay(back).into(),
            (ModalBlock::Content(front), ModalBlock::Content(back)) => front.overlay(back).into(),
            (ModalBlock::Empty(front), ModalBlock::Content(back)) => {
                let width = cmp::max(front.width, back.width());
                let height = cmp::max(front.height, back.height());
                let front = front
                    .pad_to_width_at_right(width)
                    .pad_to_height_at_bottom(height);
                let back = back
                    .pad_to_width_at_right(width)
                    .pad_to_height_at_bottom(height);
                // The height of the empty block cannot be zero here, so the
                // fill cannot fail.
                front.fill(Grapheme::SPACE).unwrap().overlay(back).into()
            }
            (ModalBlock::Content(front), ModalBlock::Empty(back)) => {
                let width = cmp::max(front.width(), back.width);
                let height = cmp::max(front.height(), back.height);
                let front = front
                    .pad_to_width_at_right(width)
                    .pad_to_height_at_bottom(height);
                let back = back
                    .pad_to_width_at_right(width)
                    .pad_to_height_at_bottom(height);
                // The height of the empty block cannot be zero here, so the
                // fill cannot fail.
                front.overlay(back.fill(Grapheme::SPACE).unwrap()).into()
            }
        }
    }
}

impl<'t> ModalBlock<Cow<'t, str>> {
    pub fn into_owned(self) -> ModalBlock<Cow<'static, str>> {
        match self {
            ModalBlock::Empty(block) => ModalBlock::Empty(block),
            ModalBlock::Content(block) => ModalBlock::Content(block.into_owned()),
        }
    }
}

impl<C, S> ModalBlock<Styled<C, S>>
where
    C: AsRef<str> + Content + From<String>,
    S: Default + Style,
{
    pub fn restyle(self, style: S) -> Self {
        match self {
            ModalBlock::Empty(block) => ModalBlock::Empty(block),
            ModalBlock::Content(block) => ModalBlock::Content(block.restyle(style)),
        }
    }
}

impl<C> From<ContentBlock<C>> for ModalBlock<C>
where
    C: Content,
{
    fn from(block: ContentBlock<C>) -> Self {
        ModalBlock::Content(block)
    }
}

impl<C> From<EmptyBlock> for ModalBlock<C>
where
    C: Content,
{
    fn from(block: EmptyBlock) -> Self {
        ModalBlock::Empty(block)
    }
}

impl<C> From<Result<ContentBlock<C>, EmptyBlock>> for ModalBlock<C>
where
    C: Content,
{
    fn from(result: Result<ContentBlock<C>, EmptyBlock>) -> Self {
        match result {
            Ok(block) => ModalBlock::Content(block),
            Err(block) => ModalBlock::Empty(block),
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Block<C = String>
where
    C: Content,
{
    inner: ModalBlock<C>,
}

impl<C> Block<C>
where
    C: Content,
{
    pub fn zero() -> Self {
        Self::with_dimensions(0, 0)
    }

    pub fn with_content(content: impl Into<C>) -> Self {
        Block {
            inner: ContentBlock::from(content.into().into_lines()).into(),
        }
    }

    pub fn with_dimensions(width: usize, height: usize) -> Self {
        Block {
            inner: EmptyBlock { width, height }.into(),
        }
    }

    pub fn with_height(height: usize) -> Self {
        Self::with_dimensions(0, height)
    }

    pub fn with_width(width: usize) -> Self {
        Self::with_dimensions(width, 0)
    }

    pub fn filled<T>(width: usize, height: usize, filler: T) -> Self
    where
        Self: Fill<C, T, Output = Self>,
    {
        Self::with_dimensions(width, height).fill(filler)
    }

    pub fn aligned<V, H>(self) -> AlignedBlock<C, V, H>
    where
        V: VerticalAlignment,
        H: HorizontalAlignment,
    {
        self.into()
    }

    pub fn height(&self) -> usize {
        self.inner.height()
    }

    pub fn width(&self) -> usize {
        self.inner.width()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn into_content_or_fill(self, glyph: Grapheme) -> Result<ContentBlock<C>, EmptyBlock> {
        match self.inner {
            ModalBlock::Empty(block) => block.fill(glyph),
            ModalBlock::Content(block) => Ok(block),
        }
    }
}

impl<C> Block<C>
where
    C: Content,
{
    pub fn push(self, content: impl Into<C>) -> Self {
        Block {
            inner: self
                .into_content_or_fill(Grapheme::SPACE)
                .unwrap_or_else(|block| {
                    ContentBlock { lines: vec![] }.pad_to_width_at_right(block.width)
                })
                .push(content)
                .into(),
        }
    }
}

impl<C> Block<C>
where
    C: Content,
{
    pub fn pad_to_width_at_right(self, width: usize) -> Self {
        self.inner.pad_to_width_at_right(width).into()
    }

    pub fn pad_to_height_at_bottom(self, height: usize) -> Self {
        self.inner.pad_to_height_at_bottom(height).into()
    }

    pub fn join_left_to_right_at_top(self, right: Self) -> Self {
        self.inner.join_left_to_right_at_top(right.inner).into()
    }

    pub fn join_top_to_bottom_at_left(self, bottom: Self) -> Self {
        self.inner.join_top_to_bottom_at_left(bottom.inner).into()
    }

    pub fn overlay(self, back: Self) -> Self {
        self.inner.overlay(back.inner).into()
    }
}

impl<C> Block<C>
where
    C: Content,
{
    pub fn pad_at_left(self, width: usize) -> Self {
        let padding = Block::filled(width, self.height(), Grapheme::SPACE);
        padding.join_left_to_right_at_top(self)
    }

    pub fn pad_at_right(self, width: usize) -> Self {
        let padding = Block::filled(width, self.height(), Grapheme::SPACE);
        self.join_left_to_right_at_top(padding)
    }

    pub fn pad_at_top(self, height: usize) -> Self {
        let padding = Block::filled(self.width(), height, Grapheme::SPACE);
        padding.join_top_to_bottom_at_left(self)
    }

    pub fn pad_at_bottom(self, height: usize) -> Self {
        let padding = Block::filled(self.width(), height, Grapheme::SPACE);
        self.join_top_to_bottom_at_left(padding)
    }

    pub fn pad_to_width_at_left(self, width: usize) -> Self {
        let width = width.saturating_sub(self.width());
        self.pad_at_left(width)
    }

    pub fn pad_to_height_at_top(self, height: usize) -> Self {
        let height = height.saturating_sub(self.height());
        self.pad_at_top(height)
    }

    pub fn join_left_to_right_at_bottom(self, right: Self) -> Self {
        let height = cmp::max(self.height(), right.height());
        self.pad_to_height_at_top(height)
            .join_left_to_right_at_top(right.pad_to_height_at_top(height))
    }

    pub fn join_top_to_bottom_at_right(self, bottom: Self) -> Self {
        let width = cmp::max(self.width(), bottom.width());
        self.pad_to_width_at_left(width)
            .join_top_to_bottom_at_left(bottom.pad_to_width_at_left(width))
    }
}

impl<'t> Block<Cow<'t, str>> {
    pub fn into_owned(self) -> Block<Cow<'static, str>> {
        Block {
            inner: self.inner.into_owned(),
        }
    }
}

impl<C, S> Block<Styled<C, S>>
where
    C: AsRef<str> + Content + From<String>,
    S: Default + Style,
{
    pub fn restyle(self, style: S) -> Self {
        Block {
            inner: self.inner.restyle(style),
        }
    }
}

impl<C> Fill<C, C> for Block<C>
where
    C: Content,
{
    type Output = Self;

    fn fill(self, content: C) -> Self::Output {
        let block = EmptyBlock {
            width: self.width(),
            height: self.height(),
        };
        Block {
            inner: match block.fill(content) {
                Ok(block) => block.into(),
                Err(block) => block.into(),
            },
        }
    }
}

impl<'t, C> Fill<C, Grapheme<'t>> for Block<C>
where
    C: Content,
{
    type Output = Self;

    fn fill(self, glyph: Grapheme<'t>) -> Self::Output {
        let block = EmptyBlock {
            width: self.width(),
            height: self.height(),
        };
        Block {
            inner: match block.fill(glyph) {
                Ok(block) => block.into(),
                Err(block) => block.into(),
            },
        }
    }
}

impl<C> From<ModalBlock<C>> for Block<C>
where
    C: Content,
{
    fn from(block: ModalBlock<C>) -> Self {
        Block { inner: block }
    }
}

impl<C> Render for Block<C>
where
    C: Content,
{
    fn render(&self) -> Cow<str> {
        match self.inner {
            ModalBlock::Empty(_) => "".into(),
            ModalBlock::Content(ref block) => block
                .lines
                .iter()
                .fold(String::new(), |mut output, content| {
                    output.push_str(content.render().trim_end());
                    output.push('\n');
                    output
                })
                .into(),
        }
    }
}

pub trait AxialBlock<C, A>: Sized
where
    C: Content,
    A: Axis,
{
    fn filled_along<T>(length: usize, width: usize, filler: T) -> Self
    where
        Block<C>: Fill<C, T, Output = Block<C>>;

    fn join_along(self, other: Self) -> Self;
}

pub trait AxialBlockOf {
    fn filled_along_of<C, A, T>(length: usize, width: usize, filler: T) -> Self
    where
        Self: AxialBlock<C, A>,
        Block<C>: Fill<C, T, Output = Block<C>>,
        C: Content,
        A: Axis,
    {
        Self::filled_along(length, width, filler)
    }

    fn join_along_of<C, A>(self, other: Self) -> Self
    where
        Self: AxialBlock<C, A>,
        C: Content,
        A: Axis,
    {
        self.join_along(other)
    }
}

impl<T> AxialBlockOf for T {}

pub trait HorizontalBlock: Sized {
    fn pad_horizontal(self, width: usize) -> Self;

    fn pad_horizontal_opposite(self, width: usize) -> Self;

    fn pad_to_width(self, width: usize) -> Self;

    fn pad_to_width_opposite(self, width: usize) -> Self;

    fn join_top_to_bottom(self, bottom: Self) -> Self;

    fn join_top_to_bottom_opposite(self, bottom: Self) -> Self;
}

pub trait VerticalBlock: Sized {
    fn pad_vertical(self, height: usize) -> Self;

    fn pad_vertical_opposite(self, height: usize) -> Self;

    fn pad_to_height(self, height: usize) -> Self;

    fn pad_to_height_opposite(self, height: usize) -> Self;

    fn join_left_to_right(self, other: Self) -> Self;

    fn join_left_to_right_opposite(self, other: Self) -> Self;
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AlignedBlock<C = String, V = Top, H = Left>
where
    C: Content,
    V: VerticalAlignment,
    H: HorizontalAlignment,
{
    block: Block<C>,
    phantom: PhantomData<(*const V, *const H)>,
}

impl<C, V, H> AlignedBlock<C, V, H>
where
    C: Content,
    V: VerticalAlignment,
    H: HorizontalAlignment,
{
    pub fn unaligned(self, f: impl FnOnce(Block<C>) -> Block<C>) -> Self {
        AlignedBlock {
            block: f(self.block),
            phantom: PhantomData,
        }
    }
}

impl<C, V, H> AsRef<Block<C>> for AlignedBlock<C, V, H>
where
    C: Content,
    V: VerticalAlignment,
    H: HorizontalAlignment,
{
    fn as_ref(&self) -> &Block<C> {
        &self.block
    }
}

impl<C, V, H> AxialBlock<C, TopBottom> for AlignedBlock<C, V, H>
where
    Self: HorizontalBlock,
    C: Content,
    V: VerticalAlignment,
    H: HorizontalAlignment,
{
    fn filled_along<T>(length: usize, width: usize, filler: T) -> Self
    where
        Block<C>: Fill<C, T, Output = Block<C>>,
    {
        Block::filled(width, length, filler).into()
    }

    fn join_along(self, bottom: Self) -> Self {
        self.join_top_to_bottom(bottom)
    }
}

impl<C, V, H> AxialBlock<C, LeftRight> for AlignedBlock<C, V, H>
where
    Self: VerticalBlock,
    C: Content,
    V: VerticalAlignment,
    H: HorizontalAlignment,
{
    fn filled_along<T>(length: usize, width: usize, filler: T) -> Self
    where
        Block<C>: Fill<C, T, Output = Block<C>>,
    {
        Block::filled(length, width, filler).into()
    }

    fn join_along(self, right: Self) -> Self {
        self.join_left_to_right(right)
    }
}

impl<C, V, H> Deref for AlignedBlock<C, V, H>
where
    C: Content,
    V: VerticalAlignment,
    H: HorizontalAlignment,
{
    type Target = Block<C>;

    fn deref(&self) -> &Self::Target {
        &self.block
    }
}

impl<C, V, H> From<Block<C>> for AlignedBlock<C, V, H>
where
    C: Content,
    V: VerticalAlignment,
    H: HorizontalAlignment,
{
    fn from(block: Block<C>) -> Self {
        AlignedBlock {
            block,
            phantom: PhantomData,
        }
    }
}

impl<C, V, H> From<AlignedBlock<C, V, H>> for Block<C>
where
    C: Content,
    V: VerticalAlignment,
    H: HorizontalAlignment,
{
    fn from(block: AlignedBlock<C, V, H>) -> Self {
        block.block
    }
}

impl<C, V> HorizontalBlock for AlignedBlock<C, V, Left>
where
    C: Content,
    V: VerticalAlignment,
{
    fn pad_horizontal(self, width: usize) -> Self {
        self.unaligned(|block| block.pad_at_left(width))
    }

    fn pad_horizontal_opposite(self, width: usize) -> Self {
        self.unaligned(|block| block.pad_at_right(width))
    }

    fn pad_to_width(self, width: usize) -> Self {
        self.unaligned(|block| block.pad_to_width_at_left(width))
    }

    fn pad_to_width_opposite(self, width: usize) -> Self {
        self.unaligned(|block| block.pad_to_width_at_right(width))
    }

    fn join_top_to_bottom(self, bottom: Self) -> Self {
        self.unaligned(|block| block.join_top_to_bottom_at_left(bottom.into()))
    }

    fn join_top_to_bottom_opposite(self, bottom: Self) -> Self {
        self.unaligned(|block| block.join_top_to_bottom_at_right(bottom.into()))
    }
}

impl<C, V> HorizontalBlock for AlignedBlock<C, V, Right>
where
    C: Content,
    V: VerticalAlignment,
{
    fn pad_horizontal(self, width: usize) -> Self {
        self.unaligned(|block| block.pad_at_right(width))
    }

    fn pad_horizontal_opposite(self, width: usize) -> Self {
        self.unaligned(|block| block.pad_at_left(width))
    }

    fn pad_to_width(self, width: usize) -> Self {
        self.unaligned(|block| block.pad_to_width_at_right(width))
    }

    fn pad_to_width_opposite(self, width: usize) -> Self {
        self.unaligned(|block| block.pad_to_width_at_left(width))
    }

    fn join_top_to_bottom(self, bottom: Self) -> Self {
        self.unaligned(|block| block.join_top_to_bottom_at_right(bottom.into()))
    }

    fn join_top_to_bottom_opposite(self, bottom: Self) -> Self {
        self.unaligned(|block| block.join_top_to_bottom_at_left(bottom.into()))
    }
}

impl<C, H> VerticalBlock for AlignedBlock<C, Bottom, H>
where
    C: Content,
    H: HorizontalAlignment,
{
    fn pad_vertical(self, height: usize) -> Self {
        self.unaligned(|block| block.pad_at_bottom(height))
    }

    fn pad_vertical_opposite(self, height: usize) -> Self {
        self.unaligned(|block| block.pad_at_top(height))
    }

    fn pad_to_height(self, height: usize) -> Self {
        self.unaligned(|block| block.pad_to_height_at_bottom(height))
    }

    fn pad_to_height_opposite(self, height: usize) -> Self {
        self.unaligned(|block| block.pad_to_height_at_top(height))
    }

    fn join_left_to_right(self, right: Self) -> Self {
        self.unaligned(|block| block.join_left_to_right_at_bottom(right.into()))
    }

    fn join_left_to_right_opposite(self, right: Self) -> Self {
        self.unaligned(|block| block.join_left_to_right_at_top(right.into()))
    }
}

impl<C, H> VerticalBlock for AlignedBlock<C, Top, H>
where
    C: Content,
    H: HorizontalAlignment,
{
    fn pad_vertical(self, height: usize) -> Self {
        self.unaligned(|block| block.pad_at_top(height))
    }

    fn pad_vertical_opposite(self, height: usize) -> Self {
        self.unaligned(|block| block.pad_at_bottom(height))
    }

    fn pad_to_height(self, height: usize) -> Self {
        self.unaligned(|block| block.pad_to_height_at_top(height))
    }

    fn pad_to_height_opposite(self, height: usize) -> Self {
        self.unaligned(|block| block.pad_to_height_at_bottom(height))
    }

    fn join_left_to_right(self, right: Self) -> Self {
        self.unaligned(|block| block.join_left_to_right_at_top(right.into()))
    }

    fn join_left_to_right_opposite(self, right: Self) -> Self {
        self.unaligned(|block| block.join_left_to_right_at_bottom(right.into()))
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use crate::block::{Block, Fill};
    use crate::content::{Style as Transform, Styled};
    use crate::Render;

    #[test]
    fn block_empty() {
        let block = <Block>::zero();
        assert!(block.is_empty());

        let block = <Block>::with_width(1);
        assert!(block.is_empty());

        let block = <Block>::with_height(1);
        assert!(block.is_empty());

        let block = <Block>::with_width(1).join_top_to_bottom_at_left(Block::with_height(1));
        assert!(block.is_empty());

        let mut block = <Block>::zero();
        block = block.push("");
        assert!(!block.is_empty());
    }

    #[test]
    fn block_styled_overlay() {
        #[derive(Clone, Copy, Debug, Default)]
        struct Style(colored::Style);

        impl Transform for Style {
            fn apply<'t>(&self, text: &'t str) -> Cow<'t, str> {
                self.0.style(text).to_string().into()
            }
        }

        type Content<'t> = Styled<Cow<'t, str>, Style>;

        let x = Block::<Content>::with_content(Content::new(
            Style(colored::style().red().on_green()),
            "rrrrrrrrrr",
        ));
        let y = Block::with_content(Content::new(
            Style(colored::style().green().on_red().bold()),
            "gg gg\ngg gg",
        ));
        //let z = y.overlay(x);
        //let z = y.fill(Grapheme::from('g')).overlay(x);
        let z = y
            .fill(Content::new(
                Style(colored::style().green().on_red().bold()),
                "abcd\nba",
            ))
            .overlay(x);
        println!("{}", z.render());
    }
}
