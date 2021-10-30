pub mod align;
pub mod block;
pub mod content;

use std::borrow::Cow;
use std::cmp;

pub use crate::block::Block;
pub use crate::content::{Style, Styled};

pub mod prelude {
    pub use crate::align::{
        AxisAligned, AxisAlignedOf as _, QuadrantAligned, QuadrantAlignedOf as _,
    };
    pub use crate::block::{AxialBlock, AxialBlockOf as _, Fill, HorizontalBlock, VerticalBlock};
    pub use crate::Render;
}

trait IntegerExt: Sized {
    fn div_ceiling(self, b: Self) -> Self;

    fn sub_or_zero(self, b: Self) -> Self;
}

impl IntegerExt for usize {
    fn div_ceiling(self, b: Self) -> Self {
        let a = self;
        (0..a).step_by(b).len()
    }

    fn sub_or_zero(self, b: Self) -> Self {
        let a = self;
        a - cmp::min(a, b)
    }
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
