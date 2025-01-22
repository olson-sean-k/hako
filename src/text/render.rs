use std::fmt::{self, Arguments, Debug, Display, Formatter, Write};
use std::marker::PhantomData;

use crate::text::style::{AnsiPrefix, Style};

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

#[derive(Debug)]
pub(crate) struct AsDisplay<'r, T, S = ()> {
    text: &'r T,
    phantom: PhantomData<fn() -> S>,
}

impl<'r, T, S> From<&'r T> for AsDisplay<'r, T, S> {
    fn from(text: &'r T) -> Self {
        AsDisplay {
            text,
            phantom: PhantomData,
        }
    }
}

impl<'r, T, S> Display for AsDisplay<'r, T, S>
where
    T: Render<S>,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let mut context = RenderContext::default();
        self.text.fmt(formatter, &mut context)
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

#[derive(Debug)]
pub struct RenderContext<S> {
    nodes: Vec<RenderNode<S>>,
}

impl<S> RenderContext<S> {
    pub fn nodes(&self) -> &[RenderNode<S>] {
        self.nodes.as_slice()
    }

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
}

impl<S> RenderContext<S>
where
    S: AnsiPrefix,
{
    pub fn fmt_with_ansi_fence(
        &self,
        formatter: &mut Formatter,
        text: impl Display,
    ) -> fmt::Result {
        Style::fmt_with_ansi_fence(self.nodes().iter().map(|node| &node.style), formatter, text)
    }
}

impl<T> Default for RenderContext<T> {
    fn default() -> Self {
        RenderContext {
            nodes: Vec::default(),
        }
    }
}
