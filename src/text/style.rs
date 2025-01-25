use std::borrow::Borrow;
use std::fmt::{self, Display, Formatter, Write};

use crate::env::StyleEncoding;
use crate::text::annotation::AnnotatedText;
use crate::text::morphology::{FlexKind, Grapheme};
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

    // TODO: Unfortunately, it seems that no ANSI styling crates provide this information. For now,
    //       styles report the most basic support so as not to be discarded when rendering to an
    //       output stream that supports ANSI styling. Implement this properly when possible.
    fn encoding(&self) -> StyleEncoding {
        StyleEncoding::ANSI4
    }
}

impl AnsiPrefix for () {
    #[inline(always)]
    fn fmt(&self, _: &mut Formatter) -> fmt::Result {
        Ok(())
    }

    fn encoding(&self) -> StyleEncoding {
        StyleEncoding::NONE
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

    fn encoding(&self) -> StyleEncoding {
        match self {
            Some(ref prefix) => prefix.encoding(),
            _ => StyleEncoding::NONE,
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

    fn display(&self) -> impl '_ + Display {
        T::display(*self)
    }

    fn encoding(&self) -> StyleEncoding {
        T::encoding(*self)
    }
}

impl<'a, T> AnsiPrefix for &'a mut T
where
    T: AnsiPrefix,
{
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        T::fmt(*self, formatter)
    }

    fn display(&self) -> impl '_ + Display {
        T::display(*self)
    }

    fn encoding(&self) -> StyleEncoding {
        T::encoding(*self)
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

    pub fn fmt_with_ansi_fence<'t, I>(
        styles: I,
        formatter: &mut Formatter,
        text: impl IntoIterator<Item = Grapheme<'t>>,
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
        // Writing each grapheme is generally much less efficient than writing a buffer. In
        // particular, it is important that write targets are buffered, otherwise each write may
        // immediately request I/O from the operating system! When writing to standard outputs, a
        // `BufWriter` or something similar is very important for performance.
        for grapheme in text {
            write!(formatter, "{}", grapheme.as_ref())?;
        }
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
