use crate::align::{Axial, AxiallyAligned, Axis, ContraAxial, OrthogonalOrigin};
use crate::block::{Block, Fill, Join, WithLength};
use crate::content::{Cell, Content, FromCell};

pub trait AxialPalette {
    type Output;

    fn aligned_at<A>(self) -> Self::Output
    where
        A: Axis;
}

impl<T> AxialPalette for Axial<T>
where
    T: Clone,
{
    type Output = T;

    fn aligned_at<A>(self) -> Self::Output
    where
        A: Axis,
    {
        AxiallyAligned::aligned_at::<A>(&self).clone()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TerminalCell<G>
where
    G: Cell,
{
    Only(G),
    StartEnd(G, G),
}

impl<G> TerminalCell<G>
where
    G: Cell,
{
    pub fn only_or_else<'a>(&'a self, mut f: impl FnMut(&'a G, &'a G) -> &'a G) -> &G {
        match self {
            TerminalCell::Only(ref x) => x,
            TerminalCell::StartEnd(ref start, ref end) => f(start, end),
        }
    }

    pub fn start(&self) -> &G {
        self.only_or_else(|start, _| start)
    }

    pub fn end(&self) -> &G {
        self.only_or_else(|_, end| end)
    }
}

impl<G> From<G> for TerminalCell<G>
where
    G: Cell,
{
    fn from(only: G) -> Self {
        TerminalCell::Only(only)
    }
}

impl<G> From<(G, G)> for TerminalCell<G>
where
    G: Cell,
{
    fn from((start, end): (G, G)) -> Self {
        TerminalCell::StartEnd(start, end)
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

#[derive(Clone, Copy, Debug)]
pub struct LinePalette<G>
where
    G: Cell,
{
    pub only: G,
    pub middle: G,
    pub terminal: TerminalCell<G>,
}

impl<G> LinePalette<G>
where
    G: Cell + Clone,
{
    pub fn uniform(cell: G) -> Self {
        LinePalette {
            only: cell.clone(),
            middle: cell.clone(),
            terminal: TerminalCell::Only(cell),
        }
    }
}

impl<G> AxialPalette for LinePalette<G>
where
    G: Cell,
{
    type Output = Self;

    fn aligned_at<A>(self) -> Self::Output
    where
        A: Axis,
    {
        self
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
