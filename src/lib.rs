mod breadth;
mod slice;

pub mod align;
pub mod block;
pub mod content;
pub mod text;

use std::borrow::Cow;
use std::io::{self, Write};

pub use crate::block::Block;
pub use crate::content::{Style, Styled};

pub mod prelude {
    pub use crate::align::{AxialEnvelope as _, HorizontalEnvelope as _, VerticalEnvelope as _};
    pub use crate::block::Fill as _;
    pub use crate::Render as _;
}

mod sealed {
    pub trait Sealed {}
}

pub trait Render {
    fn render_into(&self, target: &mut impl Write) -> io::Result<()> {
        target.write_all(self.render().as_bytes())
    }

    fn render(&self) -> Cow<str>;
}

impl<'t> Render for Cow<'t, str> {
    fn render(&self) -> Cow<str> {
        self.clone()
    }
}

impl Render for String {
    fn render(&self) -> Cow<str> {
        self.into()
    }
}

pub trait IntoWritten {
    type Written;

    fn into_written(self) -> Self::Written;
}

impl<'t> IntoWritten for Cow<'t, str> {
    type Written = String;

    fn into_written(self) -> Self::Written {
        self.into_owned()
    }
}

impl IntoWritten for String {
    type Written = Self;

    fn into_written(self) -> Self::Written {
        self
    }
}

impl<'t> IntoWritten for &'t String {
    type Written = String;

    fn into_written(self) -> Self::Written {
        self.clone()
    }
}

impl<'t> IntoWritten for &'t str {
    type Written = String;

    fn into_written(self) -> Self::Written {
        self.into()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MoveCow<T>
where
    T: IntoWritten,
{
    Unwritten(T),
    Written(T::Written),
}

impl<'t, T> From<MoveCow<T>> for Cow<'t, str>
where
    Self: From<T> + From<T::Written>,
    T: IntoWritten,
{
    fn from(text: MoveCow<T>) -> Self {
        match text {
            MoveCow::Unwritten(text) => text.into(),
            MoveCow::Written(text) => text.into(),
        }
    }
}

impl<T> From<MoveCow<T>> for String
where
    Self: From<T> + From<T::Written>,
    T: IntoWritten,
{
    fn from(text: MoveCow<T>) -> Self {
        match text {
            MoveCow::Unwritten(text) => text.into(),
            MoveCow::Written(text) => text.into(),
        }
    }
}

impl<'t> TryFrom<MoveCow<&'t str>> for &'t str {
    type Error = ();

    fn try_from(text: MoveCow<&'t str>) -> Result<Self, Self::Error> {
        match text {
            MoveCow::Unwritten(text) => Ok(text),
            _ => Err(()),
        }
    }
}
