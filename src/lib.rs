mod slice;

pub mod align;
pub mod block;
pub mod content;
pub mod cow;
pub mod text;

use std::borrow::Cow;
use std::io::{self, Write};

pub use crate::block::Block;
pub use crate::content::{Style, Styled};

pub mod prelude {
    pub use crate::align::{AxialEnvelope as _, HorizontalEnvelope as _, VerticalEnvelope as _};
    pub use crate::block::Fill as _;
}

mod sealed {
    pub trait Sealed {}
}

// TODO: Remove this in favor of `crate::render::text::Render`.
pub trait Render {
    fn render(&self) -> Cow<'_, str>;

    fn render_into(&self, target: &mut impl Write) -> io::Result<()> {
        target.write_all(self.render().as_bytes())
    }
}

impl<'t> Render for Cow<'t, str> {
    fn render(&self) -> Cow<'_, str> {
        self.clone()
    }
}

impl Render for String {
    fn render(&self) -> Cow<'_, str> {
        self.into()
    }
}
