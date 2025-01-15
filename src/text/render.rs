use std::fmt::{self, Debug, Display, Formatter};
use std::marker::PhantomData;

use crate::text::style::{AnsiPrefix, AnsiSuffix, Style};

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

// TODO: Most style types are likely inexpensive to clone, but render nodes should probably store a
//       reference instead. Note though that it is possible for `S` to a reference type!
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
