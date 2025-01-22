use std::fmt::{self, Debug, Display, Formatter};

use crate::text::geometry::LinearGeometry;
use crate::text::render::{AsDisplay, Render, RenderContext, RenderNode};
use crate::text::style::{AnsiPrefix, Style};
use crate::text::{BlockText, BlockTextProjection, TryFromText};

// TODO: Prevent nested annotations.
pub trait Annotate: Sized {
    fn annotate<A>(self, annotation: A) -> AnnotatedText<Self, A>;

    fn attach<A>(self, annotation: A) -> AnnotatedText<Self, Attachment<A>>;

    fn style<S>(self, style: S) -> AnnotatedText<Self, Style<S>>
    where
        S: AnsiPrefix;
}

impl<T> Annotate for T {
    fn annotate<A>(self, annotation: A) -> AnnotatedText<Self, A> {
        AnnotatedText {
            text: self,
            annotation,
        }
    }

    fn attach<A>(self, annotation: A) -> AnnotatedText<Self, Attachment<A>> {
        self.annotate(Attachment(annotation))
    }

    fn style<S>(self, style: S) -> AnnotatedText<Self, Style<S>>
    where
        S: AnsiPrefix,
    {
        self.annotate(Style::from(style))
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Attachment<T>(pub T);

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct AnnotatedText<T, A = ()> {
    pub text: T,
    pub annotation: A,
}

impl<T, A> AnnotatedText<T, A> {
    pub const fn annotate(text: T, annotation: A) -> Self {
        AnnotatedText { text, annotation }
    }

    pub fn from_text(text: T) -> Self
    where
        A: Default,
    {
        AnnotatedText {
            text,
            annotation: Default::default(),
        }
    }

    pub fn map_text<U, F>(self, f: F) -> AnnotatedText<U, A>
    where
        F: FnOnce(T) -> U,
    {
        let AnnotatedText { text, annotation } = self;
        AnnotatedText {
            text: f(text),
            annotation,
        }
    }

    pub fn map_annotation<U, F>(self, f: F) -> AnnotatedText<T, U>
    where
        F: FnOnce(A) -> U,
    {
        let AnnotatedText { text, annotation } = self;
        AnnotatedText {
            text,
            annotation: f(annotation),
        }
    }
}

impl<T, A> AnnotatedText<Option<T>, A> {
    pub fn transpose(self) -> Option<AnnotatedText<T, A>> {
        let AnnotatedText { text, annotation } = self;
        text.map(|text| AnnotatedText { text, annotation })
    }
}

impl<T, E, A> AnnotatedText<Result<T, E>, A> {
    pub fn transpose(self) -> Result<AnnotatedText<T, A>, E> {
        let AnnotatedText { text, annotation } = self;
        text.map(|text| AnnotatedText { text, annotation })
    }
}

impl<T, A> AnnotatedText<T, Attachment<A>> {
    pub const fn attached(text: T, attachment: A) -> Self {
        AnnotatedText {
            text,
            annotation: Attachment(attachment),
        }
    }

    pub fn map_attachment<U, F>(self, f: F) -> AnnotatedText<T, Attachment<U>>
    where
        F: FnOnce(A) -> U,
    {
        self.map_annotation(|Attachment(attachment)| Attachment(f(attachment)))
    }

    pub fn attachment(&self) -> &A {
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

impl<T, S> AnnotatedText<T, Style<S>>
where
    S: AnsiPrefix,
{
    pub const fn styled(text: T, style: S) -> Self {
        AnnotatedText {
            text,
            annotation: Style::new(style),
        }
    }

    pub fn map_style<U, F>(self, f: F) -> AnnotatedText<T, Style<U>>
    where
        F: FnOnce(S) -> U,
        U: AnsiPrefix,
    {
        self.map_annotation(move |style| style.map(f))
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

// TODO: Unlike actual block text types, `AnnotatedText` must provide explicit implementations of
//       both `BlockGeometry` and `LinearGeometry`. This isn't possible, because the blanket
//       implementation of `BlockGeometry` for `LinearGeometry` types conflicts! The alternative is
//       to implement `BlockGeometry` for `AnnotatedText` where `T` is a more specific type (that
//       is not `LinearGeometry`.
//impl<T, A> BlockGeometry for AnnotatedText<T, A>
//where
//    T: BlockText + BlockGeometry,
//{
//    fn ascii_line_break_bounds(&self) -> BoundingBox<Self::Height> {
//        self.text.ascii_line_break_bounds()
//    }
//}

impl<T, A> BlockTextProjection for AnnotatedText<T, A>
where
    T: BlockText,
{
    type RawText = <T as BlockText>::RawText;
    type BlockText = T;
    type Mapped<U>
        = AnnotatedText<U, A>
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

impl<T, A> From<T> for AnnotatedText<T, A>
where
    A: Default,
{
    fn from(text: T) -> Self {
        AnnotatedText {
            text,
            annotation: A::default(),
        }
    }
}

impl<T, A> From<(T, A)> for AnnotatedText<T, A> {
    fn from((text, annotation): (T, A)) -> Self {
        AnnotatedText { text, annotation }
    }
}

impl<T, A> LinearGeometry for AnnotatedText<T, A>
where
    T: BlockText + LinearGeometry,
{
    fn width(&self) -> usize {
        self.text.width()
    }
}

impl<T, A, S> Render<S> for AnnotatedText<T, Attachment<A>>
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
// and do not compose well. Render nodes provide a basic composition mechanism that favors the most
// local styles (i.e., segment styles are applied after line styles).
impl<T, S> Render<S> for AnnotatedText<T, Style<S>>
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

impl<T, U, A> TryFromText<AnnotatedText<U, A>> for AnnotatedText<T, A>
where
    T: TryFromText<U>,
{
    // When `T` and `U` refer to the same type, `T::Error` will be `Infallible`.
    type Error = T::Error;

    fn try_from_text(annotated: AnnotatedText<U, A>) -> Result<Self, Self::Error> {
        let AnnotatedText { text, annotation } = annotated;
        T::try_from_text(text).map(move |text| AnnotatedText { text, annotation })
    }
}
