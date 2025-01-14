use std::fmt::{self, Debug, Display, Formatter};
use std::marker::PhantomData;

use crate::text::style::{AnsiPrefix, AnsiSuffix, Style};

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
pub struct AsDisplay<'r, T, S = ()> {
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

// TODO: Most style types are likely inexpensive to clone, but render nodes should probably store a
//       reference instead.
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
        for node in self.nodes() {
            node.style.as_ref().fmt(formatter)?;
        }
        write!(formatter, "{}", text)?;
        if !self.nodes().is_empty() {
            write!(formatter, "{}", AnsiSuffix)?;
        }
        Ok(())
    }
}

impl<T> Default for RenderContext<T> {
    fn default() -> Self {
        RenderContext {
            nodes: Vec::default(),
        }
    }
}
