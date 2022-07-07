use crate::align::{Oriented, Left, Right, Top, Bottom, Rotate, Quadrant, Axial, AxiallyAligned, Axis, ContraAxial, OrthogonalOrigin, AxisValue};
use crate::block::{Block, Fill, Join, WithLength};
use crate::content::{Cell, Content, FromCell};

#[derive(Clone, Copy, Debug)]
pub struct AxisVector {
    axis: AxisValue,
    length: isize,
}

impl Oriented for AxisVector {
    type Origin = Top;
}

impl Rotate<Left> for AxisVector {
    type Output = Self;

    fn rotate(self) -> Self::Output {
        use AxisValue::{TopBottom, LeftRight};

        let AxisVector { axis, length } = self;
        match axis {
            LeftRight => AxisVector {
                axis: TopBottom,
                length: -length,
            },
            TopBottom => AxisVector {
                axis: LeftRight,
                length,
            },
        }
    }
}

pub trait Uniform<T>: Sized {
    fn uniform(value: T) -> Self;
}

pub trait Brush<C, G>
where
    C: Content + FromCell<G>,
    G: Cell,
{
    fn stroke(&self) -> Stroke<C, G>;

    fn fill(&self) -> C;
}

#[derive(Clone, Copy, Debug)]
pub struct Palette<C, G>
where
    C: Content + FromCell<G>,
    G: Cell,
{
    pub stroke: Stroke<C, G>,
    pub fill: C,
}

impl<C, G> Brush<C, G> for Palette<C, G>
where
    Stroke<C, G>: Clone,
    C: Content + FromCell<G>,
    G: Cell,
{
    fn stroke(&self) -> Stroke<C, G> {
        self.stroke.clone()
    }

    fn fill(&self) -> C {
        self.fill.clone()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Stroke<C, G>
where
    C: Content + FromCell<G>,
    G: Cell,
{
    pub straight: Axial<StraightStroke<C, G>>,
    pub corner: Quadrant<CornerStroke<G>>,
}

pub type CornerStroke<G> = G;

#[derive(Clone, Copy, Debug)]
pub struct StraightStroke<C, G>
where
    C: Content + FromCell<G>,
    G: Cell,
{
    pub only: G,
    pub middle: C,
    pub end: Terminal<G>,
}

#[derive(Clone, Copy, Debug)]
pub enum Terminal<T> {
    Only(T),
    StartEnd(T, T),
}

impl<T> Terminal<T> {
    pub fn only_or_else<'a>(&'a self, mut f: impl FnMut(&'a T, &'a T) -> &'a T) -> &T {
        match self {
            Terminal::Only(ref x) => x,
            Terminal::StartEnd(ref start, ref end) => f(start, end),
        }
    }

    pub fn start(&self) -> &T {
        self.only_or_else(|start, _| start)
    }

    pub fn end(&self) -> &T {
        self.only_or_else(|_, end| end)
    }
}

impl<T> From<T> for Terminal<T> {
    fn from(only: T) -> Self {
        Terminal::Only(only)
    }
}

impl<T> From<(T, T)> for Terminal<T> {
    fn from((start, end): (T, T)) -> Self {
        Terminal::StartEnd(start, end)
    }
}

pub trait Line<A, C>
where
    A: Axis,
    C: Content,
{
    fn line<G, P>(length: usize, palette: &P) -> Self
    where
        C: FromCell<G>,
        G: Cell + Clone,
        P: AxialPalette<Output = LinePalette<G>> + Clone;
}

impl<A, C> Line<A, C> for Block<C>
where
    Self: Join<A, OrthogonalOrigin<A>> + WithLength<A>,
    OrthogonalOrigin<A>: ContraAxial<A>,
    A: Axis,
    C: Content,
{
    fn line<G, P>(length: usize, palette: &P) -> Self
    where
        C: FromCell<G>,
        G: Cell + Clone,
        P: AxialPalette<Output = LinePalette<G>> + Clone,
    {
        let LinePalette {
            only,
            middle,
            terminal,
        } = palette.clone().aligned_at::<A>();
        match length {
            0 => Block::zero(),
            1 => Block::with_content(C::from_cell(only)),
            _ => Block::with_content(C::from_cell(terminal.start().clone()))
                .join(Block::with_length(length - 2, 1).fill(C::from_cell(middle)))
                .join(Block::with_content(C::from_cell(terminal.end().clone()))),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::align::{Axial, LeftRight, TopBottom};
    use crate::block::Block;
    use crate::primitive::{Line, LinePalette};
    use crate::Render;

    use std::borrow::Cow;

    #[test]
    fn line() {
        let block: Block<Cow<str>> = Line::<LeftRight, _>::line(
            5,
            &LinePalette {
                only: '-',
                middle: '-',
                terminal: ('<', '>').into(),
            },
        );
        assert_eq!(block.render(), "<--->\n");

        let block: Block = Line::<LeftRight, _>::line(
            5,
            &Axial {
                horizontal: LinePalette::uniform('-'),
                vertical: LinePalette::uniform('|'),
            },
        );
        assert_eq!(block.render(), "-----\n");

        let block: Block = Line::<TopBottom, _>::line(3, &LinePalette::uniform('|'));
        assert_eq!(block.render(), "|\n|\n|\n");
    }
}
