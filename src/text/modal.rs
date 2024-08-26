use ModalWidth::{Narrow, Wide};

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
        matches!(self, Narrow(_))
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
