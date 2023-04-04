pub mod align;
pub mod block;
pub mod content;

use std::borrow::Cow;
use std::io::{self, Write};

pub use crate::block::Block;
pub use crate::content::{Style, Styled};

pub mod prelude {
    pub use crate::align::{AxialEnvelope as _, HorizontalEnvelope as _, VerticalEnvelope as _};
    pub use crate::block::Fill as _;
    pub use crate::Render as _;
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
