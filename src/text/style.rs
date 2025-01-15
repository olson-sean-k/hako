use std::fmt::{self, Display, Formatter};

use crate::text::annotation::AnnotatedText;

const ANSI_RESET_ESCAPE_SEQUENCE: &str = "\u{1b}[0m";

pub type StyledText<T, S> = AnnotatedText<T, Style<S>>;

pub trait AnsiPrefix {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result;
}

impl AnsiPrefix for () {
    #[inline(always)]
    fn fmt(&self, _: &mut Formatter) -> fmt::Result {
        Ok(())
    }
}

impl<S> AnsiPrefix for Option<S>
where
    S: AnsiPrefix,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Some(ref fence) => fence.fmt(formatter),
            _ => Ok(()),
        }
    }
}

impl<'a, T> AnsiPrefix for &'a T
where
    T: AnsiPrefix,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        T::fmt(*self, formatter)
    }
}

impl<'a, T> AnsiPrefix for &'a mut T
where
    T: AnsiPrefix,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        T::fmt(*self, formatter)
    }
}

#[cfg(feature = "owo-colors")]
#[cfg_attr(docsrs, doc(cfg(feature = "owo-colors")))]
impl AnsiPrefix for owo_colors::Style {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        owo_colors::Style::fmt_prefix(self, formatter)
    }
}

#[derive(Debug, Default)]
pub struct AnsiSuffix;

impl Display for AnsiSuffix {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", ANSI_RESET_ESCAPE_SEQUENCE)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(transparent)]
pub struct Style<S>(pub(crate) S);

impl<S> Style<S>
where
    S: AnsiPrefix,
{
    pub const fn new(fence: S) -> Self {
        Style(fence)
    }
}

impl<S> AsRef<S> for Style<S>
where
    S: AnsiPrefix,
{
    fn as_ref(&self) -> &S {
        &self.0
    }
}

impl<S> From<S> for Style<S>
where
    S: AnsiPrefix,
{
    fn from(style: S) -> Self {
        Style::new(style)
    }
}
