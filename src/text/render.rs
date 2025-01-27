use ascii::AsciiChar;
use std::fmt::{self, Arguments, Debug, Display, Formatter, Write};
use std::marker::PhantomData;

use crate::env::{Detected, Encoding, Query, StyleEncoding, TextEncoding};
use crate::text::morphology::Grapheme;
use crate::text::style::{AnsiPrefix, Style};
use crate::text::IteratorExt as _;

// TODO: Though it may introduce some tricky indirection, it may be useful for `Render`
//       implementations and `display` functions to only require style types that can differ but
//       coerce to a common type. This would support block text types where, for example, one type
//       moves a style type `S` while a composed type borrows a style type `&'_ S`.
pub trait Render<S> {
    fn fmt(&self, formatter: &mut Formatter, context: &mut RenderContext<S>) -> fmt::Result;
}

impl<T, S> Render<S> for &'_ T
where
    T: Render<S>,
{
    fn fmt(&self, formatter: &mut Formatter, context: &mut RenderContext<S>) -> fmt::Result {
        T::fmt(self, formatter, context)
    }
}

pub trait DisplayStyle {
    type Style: AnsiPrefix;
}

#[derive(Debug)]
pub struct DisplayProxy<'t, T, S = ()> {
    text: &'t T,
    phantom: PhantomData<fn() -> S>,
}

impl<'t, T, S> DisplayProxy<'t, T, S> {
    pub(crate) fn from_text(text: &'t T) -> Self {
        DisplayProxy {
            text,
            phantom: PhantomData,
        }
    }
}

impl<'t, T, S> DisplayProxy<'t, T, S>
where
    T: Render<S>,
    S: 't + Clone,
{
    pub fn default(self) -> impl 't + Display {
        self.with(RenderContext::default())
    }

    pub fn detected<Q>(self, query: impl Query<Q>) -> impl 't + Display
    where
        RenderContext<S>: Detected<Q>,
        Q: 't + Copy,
    {
        self.with(RenderContext::detected(query))
    }

    pub fn with(self, context: RenderContext<S>) -> impl 't + Display {
        FmtWith(move |formatter| {
            let mut context = context.clone();
            self.text.fmt(formatter, &mut context)
        })
    }
}

#[derive(Clone, Copy)]
pub(crate) struct FmtWith<F>(pub F)
where
    F: Fn(&mut Formatter<'_>) -> fmt::Result;

impl<F> Debug for FmtWith<F>
where
    F: Fn(&mut Formatter<'_>) -> fmt::Result,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        (self.0)(formatter)
    }
}

impl<F> Display for FmtWith<F>
where
    F: Fn(&mut Formatter<'_>) -> fmt::Result,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        (self.0)(formatter)
    }
}

#[derive(Debug)]
pub struct Monitor<W> {
    write: W,
    has_observed_writes: bool,
}

impl<W> Monitor<W> {
    pub fn new(write: W) -> Self {
        Monitor {
            write,
            has_observed_writes: false,
        }
    }

    pub fn into_monitored(self) -> W {
        self.write
    }

    pub fn has_observed_writes(&self) -> bool {
        self.has_observed_writes
    }
}

// TODO: `has_observed_writes` observes whether or not a write function has been called, but not if
//       anything has actually been written (i.e., it does not consider empty inputs). Test this
//       and determine the best behavior here. Note that it may be difficult to examine `Arguments`
//       in `write_fmt`.
impl<W> Write for Monitor<W>
where
    W: Write,
{
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.has_observed_writes = true;
        self.write.write_str(text)
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        self.has_observed_writes = true;
        self.write.write_char(c)
    }

    fn write_fmt(&mut self, arguments: Arguments<'_>) -> fmt::Result {
        self.has_observed_writes = true;
        self.write.write_fmt(arguments)
    }
}

// TODO: Most style types are likely inexpensive to clone, but render nodes should probably store a
//       reference instead. Note though that it is possible that `S` is a reference type!
//       Ultimately, a `Reborrow<Target = S>` trait is likely the best way to ensure that a direct
//       reference is always stored.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderNode<S> {
    pub style: Style<S>,
}

#[derive(Clone, Debug)]
pub struct RenderContext<S> {
    encoding: Encoding,
    ascii_replacement_character: AsciiChar,
    nodes: Vec<RenderNode<S>>,
}

impl<S> RenderContext<S> {
    pub fn push_node_and_fmt<T, F>(&mut self, formatter: &mut Formatter, f: F) -> fmt::Result
    where
        T: Render<S>,
        F: FnOnce() -> (T, RenderNode<S>),
    {
        let (text, node) = f();
        self.nodes.push(node);
        let result = text.fmt(formatter, self);
        self.nodes.pop().unwrap();
        result
    }

    pub fn nodes(&self) -> &[RenderNode<S>] {
        self.nodes.as_slice()
    }
}

impl<S> RenderContext<S>
where
    S: AnsiPrefix,
{
    pub fn fmt_with_ansi_fence<'t>(
        &self,
        encoding: TextEncoding,
        formatter: &mut Formatter,
        text: impl IntoIterator<Item = Grapheme<'t>>,
    ) -> fmt::Result {
        let styles = self
            .nodes()
            .iter()
            .map(|node| &node.style)
            .filter(|style| style.as_ref().encoding().is_in(&self.encoding.style));
        if encoding.is_in(&self.encoding.text) {
            Style::fmt_with_ansi_fence(styles, formatter, text)
        }
        else {
            Style::fmt_with_ansi_fence(
                styles,
                formatter,
                text.into_iter()
                    .map_unicode_to_ascii(|_| self.ascii_replacement_character),
            )
        }
    }
}

impl<S> Default for RenderContext<S> {
    fn default() -> Self {
        RenderContext {
            encoding: Encoding {
                style: StyleEncoding::NONE,
                text: TextEncoding::ASCII,
            },
            ascii_replacement_character: AsciiChar::Question,
            nodes: Vec::default(),
        }
    }
}

impl<S> Detected<Encoding> for RenderContext<S> {
    fn detected(encoding: impl Query<Encoding>) -> Self {
        encoding.query().into()
    }
}

impl<S> From<Encoding> for RenderContext<S> {
    fn from(encoding: Encoding) -> Self {
        RenderContext {
            encoding,
            ..Default::default()
        }
    }
}
