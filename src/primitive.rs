use crate::align::{Axial, AxiallyAligned, Axis, ContraAxial, OrthogonalOrigin};
use crate::block::{self, Block, Fill};
use crate::content::{Cell, Content, FromCell};

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
    pub fn aligned<A>(only: Axial<T>, middle: Axial<T>, terminal: Axial<TerminalCell<T>>) -> Self
    where
        A: Axis,
    {
        LinePalette {
            only: only.aligned_at::<A>().clone(),
            middle: middle.aligned_at::<A>().clone(),
            terminal: terminal.aligned_at::<A>().clone(),
        }
    }

    pub fn uniform(cell: T) -> Self {
        LinePalette {
            only: cell.clone(),
            middle: cell.clone(),
            terminal: TerminalCell::Only(cell),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Line<T>
where
    T: Cell,
{
    pub length: usize,
    pub palette: LinePalette<T>,
}

impl<T> Line<T>
where
    T: Cell + Clone,
{
    pub fn into_block<A, C>(self) -> Block<C>
    where
        Block<C>: FromLine<A, C>,
        A: Axis,
        C: Content + FromCell<T>,
    {
        let Line {
            length,
            palette:
                LinePalette {
                    only,
                    middle,
                    terminal,
                },
        } = self;
        match length {
            0 => Block::zero(),
            1 => Block::filled(1, 1, C::from_cell(only)),
            _ => Block::with_content(C::from_cell(terminal.start().clone()))
                .join(Block::with_length(length - 2, 1).fill(C::from_cell(middle)))
                .join(Block::with_content(C::from_cell(terminal.end().clone()))),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::align::LeftRight;
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
    }
}
