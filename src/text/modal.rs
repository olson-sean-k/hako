use ModalText::{Blank, Content};
use ModalWidth::{Narrow, Wide};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ModalText<B, C> {
    Blank(B),
    Content(C),
}

impl<B, C> ModalText<B, C> {
    pub fn map_blank<U, F>(self, f: F) -> ModalText<U, C>
    where
        F: FnOnce(B) -> U,
    {
        match self {
            Blank(blank) => Blank(f(blank)),
            Content(content) => Content(content),
        }
    }

    pub fn map_content<U, F>(self, f: F) -> ModalText<B, U>
    where
        F: FnOnce(C) -> U,
    {
        match self {
            Blank(blank) => Blank(blank),
            Content(content) => Content(f(content)),
        }
    }

    pub fn blank(self) -> Option<B> {
        match self {
            Blank(blank) => Some(blank),
            _ => None,
        }
    }

    pub fn content(self) -> Option<C> {
        match self {
            Content(content) => Some(content),
            _ => None,
        }
    }

    pub fn as_ref(&self) -> ModalText<&B, &C> {
        match self {
            Blank(ref blank) => Blank(blank),
            Content(ref content) => Content(content),
        }
    }

    pub fn is_blank(&self) -> bool {
        matches!(self, Blank(_))
    }

    pub fn is_content(&self) -> bool {
        matches!(self, Content(_))
    }
}

impl<T> ModalText<T, T> {
    pub fn into_inner(self) -> T {
        match self {
            Blank(inner) | Content(inner) => inner,
        }
    }

    pub fn map<U, F>(self, f: F) -> ModalText<U, U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Blank(blank) => Blank(f(blank)),
            Content(content) => Content(f(content)),
        }
    }
}

impl<B, C, T> Iterator for ModalText<B, C>
where
    B: Iterator<Item = T>,
    C: Iterator<Item = T>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ModalText::Blank(ref mut blank) => blank.next(),
            ModalText::Content(ref mut content) => content.next(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ModalWidth<N, W> {
    Narrow(N),
    Wide(W),
}

impl<N, W> ModalWidth<N, W> {
    pub fn map_narrow<U, F>(self, f: F) -> ModalWidth<U, W>
    where
        F: FnOnce(N) -> U,
    {
        match self {
            Narrow(narrow) => Narrow(f(narrow)),
            Wide(wide) => Wide(wide),
        }
    }

    pub fn map_wide<U, F>(self, f: F) -> ModalWidth<N, U>
    where
        F: FnOnce(W) -> U,
    {
        match self {
            Narrow(narrow) => Narrow(narrow),
            Wide(wide) => Wide(f(wide)),
        }
    }

    pub fn narrow(self) -> Option<N> {
        match self {
            Narrow(narrow) => Some(narrow),
            _ => None,
        }
    }

    pub fn wide(self) -> Option<W> {
        match self {
            Wide(wide) => Some(wide),
            _ => None,
        }
    }

    pub fn as_ref(&self) -> ModalWidth<&N, &W> {
        match self {
            Narrow(ref narrow) => Narrow(narrow),
            Wide(ref wide) => Wide(wide),
        }
    }

    pub fn is_narrow(&self) -> bool {
        matches!(self, Narrow(_))
    }

    pub fn is_wide(&self) -> bool {
        matches!(self, Wide(_))
    }
}

impl<T> ModalWidth<T, T> {
    pub fn into_inner(self) -> T {
        match self {
            Narrow(inner) | Wide(inner) => inner,
        }
    }

    pub fn map<U, F>(self, f: F) -> ModalWidth<U, U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Narrow(narrow) => Narrow(f(narrow)),
            Wide(wide) => Wide(f(wide)),
        }
    }
}
