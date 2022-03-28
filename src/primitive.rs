use crate::align::{Axial, AxiallyAligned, Axis, ContraAxial, OrthogonalOrigin};
use crate::block::{self, Block, Fill};
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
pub enum TerminalCell<T>
where
    T: Cell,
{
    Only(T),
    StartEnd(T, T),
}

impl<T> TerminalCell<T>
where
    T: Cell,
{
    pub fn only_or_else<'a>(&'a self, mut f: impl FnMut(&'a T, &'a T) -> &'a T) -> &T {
        match self {
            TerminalCell::Only(ref x) => x,
            TerminalCell::StartEnd(ref start, ref end) => f(start, end),
        }
    }

    pub fn start(&self) -> &T {
        self.only_or_else(|start, _| start)
    }

    pub fn end(&self) -> &T {
        self.only_or_else(|_, end| end)
    }
}

impl<T> From<T> for TerminalCell<T>
where
    T: Cell,
{
    fn from(only: T) -> Self {
        TerminalCell::Only(only)
    }
}

impl<T> From<(T, T)> for TerminalCell<T>
where
    T: Cell,
{
    fn from((start, end): (T, T)) -> Self {
        TerminalCell::StartEnd(start, end)
    }
}

pub trait FromLine<A, C>
where
    A: Axis,
    C: Content,
{
    fn with_length(length: usize, width: usize) -> Self;

    #[must_use]
    fn join(self, other: Self) -> Self;
}

impl<A, C> FromLine<A, C> for Block<C>
where
    Self: block::Join<A, OrthogonalOrigin<A>> + block::WithLength<A>,
    OrthogonalOrigin<A>: ContraAxial<A>,
    A: Axis,
    C: Content,
{
    fn with_length(length: usize, width: usize) -> Self {
        block::WithLength::with_length(length, width)
    }

    fn join(self, other: Self) -> Self {
        block::Join::join(self, other)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LinePalette<T>
where
    T: Cell,
{
    pub only: T,
    pub middle: T,
    pub terminal: TerminalCell<T>,
}

impl<T> LinePalette<T>
where
    T: Cell + Clone,
{
    pub fn uniform(cell: T) -> Self {
        LinePalette {
            only: cell.clone(),
            middle: cell.clone(),
            terminal: TerminalCell::Only(cell),
        }
    }
}

impl<T> AxialPalette for LinePalette<T>
where
    T: Cell,
{
    type Output = Self;

    fn aligned_at<A>(self) -> Self::Output
    where
        A: Axis,
    {
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Line<T, P = LinePalette<T>>
where
    T: Cell,
    P: AxialPalette<Output = LinePalette<T>>,
{
    pub length: usize,
    pub palette: P,
}

impl<T, P> Line<T, P>
where
    T: Cell + Clone,
    P: AxialPalette<Output = LinePalette<T>>,
{
    pub fn into_block<A, C>(self) -> Block<C>
    where
        Block<C>: FromLine<A, C>,
        A: Axis,
        C: Content + FromCell<T>,
    {
        let Line { length, palette } = self;
        let LinePalette {
            only,
            middle,
            terminal,
        } = palette.aligned_at::<A>();
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
    use crate::align::{Axial, LeftRight};
    use crate::primitive::{Line, LinePalette};

    use std::borrow::Cow;

    #[test]
    fn line() {
        let _ = Line {
            length: 5,
            palette: LinePalette {
                only: '-',
                middle: '-',
                terminal: ('<', '>').into(),
            },
        }
        .into_block::<LeftRight, Cow<str>>();

        let _ = Line {
            length: 5,
            palette: Axial {
                horizontal: LinePalette::uniform('-'),
                vertical: LinePalette::uniform('|'),
            },
        }
        .into_block::<LeftRight, Cow<str>>();
    }
}
