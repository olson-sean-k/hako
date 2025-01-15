use std::fmt::{self, Debug, Display, Formatter};

use crate::text::render::{AsDisplay, Render, RenderContext, RenderNode};
use crate::text::style::{AnsiPrefix, Style};
use crate::text::{BlockText, BlockTextProjection, MorphologyError, RawText, TryFromText};

// TODO: Prevent nested annotations.
pub trait Annotate: Sized {
    fn annotate<A>(self, annotation: A) -> Annotated<Self, A>;

    fn attach<A>(self, annotation: A) -> Annotated<Self, Attachment<A>>;

    fn style<S>(self, style: S) -> Annotated<Self, Style<S>>
    where
        S: AnsiPrefix;
}

impl<T> Annotate for T {
    fn annotate<A>(self, annotation: A) -> Annotated<Self, A> {
        Annotated {
            text: self,
            annotation,
        }
    }

    fn attach<A>(self, annotation: A) -> Annotated<Self, Attachment<A>> {
        self.annotate(Attachment(annotation))
    }

    fn style<S>(self, style: S) -> Annotated<Self, Style<S>>
    where
        S: AnsiPrefix,
    {
        self.annotate(Style::from(style))
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Attachment<T>(pub T);

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct Annotated<T, A = ()> {
    pub text: T,
    pub annotation: A,
}

impl<T, A> Annotated<T, A> {
    pub const fn annotate(text: T, annotation: A) -> Self {
        Annotated { text, annotation }
    }

    pub fn from_text(text: T) -> Self
    where
        A: Default,
    {
        Annotated {
            text,
            annotation: Default::default(),
        }
    }

    pub fn map_text<U, F>(self, f: F) -> Annotated<U, A>
    where
        F: FnOnce(T) -> U,
    {
        let Annotated { text, annotation } = self;
        Annotated {
            text: f(text),
            annotation,
        }
    }

    pub fn map_annotation<U, F>(self, f: F) -> Annotated<T, U>
    where
        F: FnOnce(A) -> U,
    {
        let Annotated { text, annotation } = self;
        Annotated {
            text,
            annotation: f(annotation),
        }
    }
}

impl<T, A> Annotated<T, Attachment<A>> {
    pub const fn inert(text: T, annotation: A) -> Self {
        Annotated {
            text,
            annotation: Attachment(annotation),
        }
    }

    pub fn annotation(&self) -> &A {
        &self.annotation.0
    }

    pub fn display<'d, S>(&'d self) -> impl 'd + Display
    where
        Self: Render<S>,
        S: 'd + AnsiPrefix,
    {
        AsDisplay::<_, S>::from(self)
    }
}

impl<T, S> Annotated<T, Style<S>>
where
    S: AnsiPrefix,
{
    pub const fn styled(text: T, style: S) -> Self {
        Annotated {
            text,
            annotation: Style::new(style),
        }
    }

    pub fn style(&self) -> &S {
        self.annotation.as_ref()
    }

    pub fn display(&self) -> impl '_ + Display
    where
        Self: Render<S>,
    {
        AsDisplay::from(self)
    }
}

impl<T, A> From<T> for Annotated<T, A>
where
    A: Default,
{
    fn from(text: T) -> Self {
        Annotated {
            text,
            annotation: A::default(),
        }
    }
}

impl<T, A> From<(T, A)> for Annotated<T, A> {
    fn from((text, annotation): (T, A)) -> Self {
        Annotated { text, annotation }
    }
}

impl<T, U, A> TryFromText<Annotated<U, A>> for Annotated<T, A>
where
    T: TryFromText<U>,
    U: RawText,
{
    fn try_from_text(annotated: Annotated<U, A>) -> Result<Self, MorphologyError> {
        let Annotated { text, annotation } = annotated;
        T::try_from_text(text).map(move |text| Annotated { text, annotation })
    }
}

impl<T, A> BlockTextProjection for Annotated<T, A>
where
    T: BlockText,
{
    type RawText = <T as BlockText>::RawText;
    type BlockText = T;
    type Mapped<U>
        = Annotated<U, A>
    where
        U: BlockText;

    fn into_block_text(self) -> Self::BlockText {
        self.text
    }

    fn map_block_text<U, F>(self, f: F) -> Self::Mapped<U>
    where
        U: BlockText,
        F: FnOnce(Self::BlockText) -> U,
    {
        self.map_text(f)
    }

    fn as_block_text(&self) -> &Self::BlockText {
        &self.text
    }

    fn as_block_text_mut(&mut self) -> &mut Self::BlockText {
        &mut self.text
    }
}

impl<T, A, S> Render<S> for Annotated<T, Attachment<A>>
where
    T: Render<S>,
{
    fn fmt(&self, formatter: &mut Formatter, context: &mut RenderContext<S>) -> fmt::Result {
        self.text.fmt(formatter, context)
    }
}

// Styled text pushes a render node with its style. Once a segment (leaf node) is reached in the
// render call tree, the stack of styles is applied in order. This guarantees that ANSI escape
// sequences are applied completely to each segment. These escape sequences act much like commands
// and do compose well. Render nodes provide a basic composition mechanism that favors the most
// local styles (i.e., segment styles are applied after line styles).
impl<T, S> Render<S> for Annotated<T, Style<S>>
where
    T: Render<S>,
    S: Clone,
{
    fn fmt(&self, formatter: &mut Formatter, context: &mut RenderContext<S>) -> fmt::Result {
        context.push_node_and_fmt(formatter, || {
            (
                &self.text,
                RenderNode {
                    style: self.annotation.clone(),
                },
            )
        })
    }
}
