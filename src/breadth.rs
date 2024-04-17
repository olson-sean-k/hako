pub use Breadth::{Narrow, Wide};

// NOTE: Remove this comment later. This general sum type is used, because this narrow vs. wide
//       dichotomony is expected to show up in various internal APIs. The biggest examples are text
//       representations (morphemes) as well as buffering and rendering (cells). Both of these must
//       make this same distinction, but likely must associate different data with each variant.

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Breadth<N, W> {
    Narrow(N),
    Wide(W),
}

impl<N, W> Breadth<N, W> {
    pub fn map_narrow<U, F>(self, f: F) -> Breadth<U, W>
    where
        F: FnOnce(N) -> U,
    {
        match self {
            Narrow(narrow) => Narrow(f(narrow)),
            Wide(wide) => Wide(wide),
        }
    }

    pub fn map_wide<U, F>(self, f: F) -> Breadth<N, U>
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

    pub fn as_ref(&self) -> Breadth<&N, &W> {
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

impl<T> Breadth<T, T> {
    pub fn into_inner(self) -> T {
        match self {
            Narrow(inner) | Wide(inner) => inner,
        }
    }

    pub fn map<U, F>(self, f: F) -> Breadth<U, U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Narrow(narrow) => Narrow(f(narrow)),
            Wide(wide) => Wide(f(wide)),
        }
    }
}
