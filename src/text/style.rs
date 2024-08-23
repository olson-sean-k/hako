use std::borrow::Cow;
use std::io::{self, Write};

use crate::text::annotation::Annotated;

pub type StyledText<T, S> = Annotated<T, Styler<S>>;

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

impl<T> Style for Option<T>
where
    T: Style,
{
    fn encode<'t>(&self, text: &'t str) -> Cow<'t, str> {
        match self {
            Some(ref style) => style.encode(text),
            _ => text.into(),
        }
    }
}

impl<'a, T> Style for &'a T
where
    T: Style,
{
    fn encode<'t>(&self, text: &'t str) -> Cow<'t, str> {
        T::encode(*self, text)
    }

    fn encode_into(&self, text: &str, target: &mut impl Write) -> io::Result<()> {
        T::encode_into(*self, text, target)
    }
}

impl<'a, T> Style for &'a mut T
where
    T: Style,
{
    fn encode<'t>(&self, text: &'t str) -> Cow<'t, str> {
        T::encode(*self, text)
    }

    fn encode_into(&self, text: &str, target: &mut impl Write) -> io::Result<()> {
        T::encode_into(*self, text, target)
    }
}

#[cfg(feature = "owo-colors")]
#[cfg_attr(docsrs, doc(cfg(feature = "owo-colors")))]
impl Style for owo_colors::Style {
    fn encode<'t>(&self, text: &'t str) -> Cow<'t, str> {
        self.style(text).to_string().into()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Styler<S>(S)
where
    S: Style;

impl<S> Styler<S>
where
    S: Style,
{
    pub const fn new(style: S) -> Self {
        Styler(style)
    }

    pub fn encode<'t>(&self, text: &'t str) -> Cow<'t, str> {
        self.0.encode(text)
    }

    pub fn encode_into(&self, text: &str, target: &mut impl Write) -> io::Result<()> {
        self.0.encode_into(text, target)
    }
}

impl<S> AsRef<S> for Styler<S>
where
    S: Style,
{
    fn as_ref(&self) -> &S {
        &self.0
    }
}

impl<S> From<S> for Styler<S>
where
    S: Style,
{
    fn from(style: S) -> Self {
        Styler::new(style)
    }
}
