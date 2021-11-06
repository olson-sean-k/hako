pub mod align;
pub mod block;
pub mod content;

use std::borrow::Cow;

pub use crate::block::Block;
pub use crate::content::{Style, Styled};

pub mod prelude {
    pub use crate::align::{HorizontallyAligned as _, VerticallyAligned as _};
    pub use crate::block::{Fill, StaticallyAligned as _};
    pub use crate::Render;
}

pub trait Render {
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
