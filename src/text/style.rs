use std::borrow::Borrow;
use std::fmt::{self, Display, Formatter, Write};

use crate::text::annotation::AnnotatedText;
use crate::text::morphology::FlexKind;
use crate::text::render::{FmtWith, Monitor};
use crate::text::{Line, Segment};

const ANSI_RESET_ESCAPE_SEQUENCE: &str = "\u{1b}[0m";

pub type StyledText<T, S> = AnnotatedText<T, Style<S>>;

pub type StyledSegment<S, T = String, M = FlexKind> = StyledText<Segment<T, M>, S>;
pub type StyledLine<S, T = Segment> = StyledText<Line<T>, S>;

pub trait AnsiPrefix {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result;

    fn display(&self) -> impl '_ + Display {
        FmtWith(|formatter| self.fmt(formatter))
    }
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
            Some(ref prefix) => prefix.fmt(formatter),
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

impl AnsiSuffix {
    pub fn fmt(formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", ANSI_RESET_ESCAPE_SEQUENCE)
    }

    pub fn display() -> impl 'static + Display {
        FmtWith(|formatter| AnsiSuffix::fmt(formatter))
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(transparent)]
pub struct Style<S>(S);

impl<S> Style<S>
where
    S: AnsiPrefix,
{
    pub const fn new(prefix: S) -> Self {
        Style(prefix)
    }

    pub fn map<U, F>(self, f: F) -> Style<U>
    where
        F: FnOnce(S) -> U,
        U: AnsiPrefix,
    {
        Style(f(self.0))
    }

    pub fn fmt_with_ansi_fence<I>(
        styles: I,
        formatter: &mut Formatter,
        text: impl Display,
    ) -> fmt::Result
    where
        I: IntoIterator,
        I::Item: Borrow<Self>,
    {
        let formatter = &mut Monitor::new(formatter);
        for style in styles {
            write!(formatter, "{}", style.borrow().as_ref().display())?;
        }
        let has_ansi_prefix = formatter.has_observed_writes();
        write!(formatter, "{}", text)?;
        if has_ansi_prefix {
            write!(formatter, "{}", AnsiSuffix::display())?;
        }
        Ok(())
    }
}

impl<S> AsRef<S> for Style<S> {
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
