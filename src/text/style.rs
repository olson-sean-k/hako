use std::borrow::Cow;
use std::io::{self, Write};

use crate::text::Annotated;
use crate::Render;

pub trait Style {
    fn encode<'t>(&self, text: &'t str) -> Cow<'t, str>;

    fn encode_into(&self, text: &str, target: &mut impl Write) -> io::Result<()> {
        target.write_all(self.encode(text).as_bytes())
    }
}

impl Style for () {
    fn encode<'t>(&self, text: &'t str) -> Cow<'t, str> {
        text.into()
    }

    fn encode_into(&self, _: &str, _: &mut impl Write) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(feature = "owo-colors")]
#[cfg_attr(docsrs, doc(cfg(feature = "owo-colors")))]
impl Style for owo_colors::Style {
    fn encode<'t>(&self, text: &'t str) -> Cow<'t, str> {
        self.style(text).to_string().into()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Styler<S>(S)
where
    S: Style;

impl<S> Styler<S>
where
    S: Style,
{
    pub fn encode<'t>(&self, text: &'t str) -> Cow<'t, str> {
        self.0.encode(text)
    }

    pub fn encode_into(&self, text: &str, target: &mut impl Write) -> io::Result<()> {
        self.0.encode_into(text, target)
    }
}

pub type StyledText<T, S> = Annotated<T, Styler<S>>;

impl<T, S> StyledText<T, S>
where
    S: Style,
{
    pub const fn styled(text: T, style: S) -> Self {
        Annotated {
            text,
            annotation: Styler(style),
        }
    }

    pub fn style(&self) -> &S {
        &self.annotation.0
    }
}

impl<T, S> Render for StyledText<T, S>
where
    T: Render,
    S: Style,
{
    fn render(&self) -> Cow<str> {
        let text = self.text.render();
        self.annotation.encode(text.as_ref()).into_owned().into()
    }

    fn render_into(&self, target: &mut impl Write) -> io::Result<()> {
        self.annotation
            .encode_into(self.text.render().as_ref(), target)
    }
}
