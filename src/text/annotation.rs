use std::borrow::Cow;
use std::fmt::Debug;
use std::io::{self, Write};

use crate::text::style::{Style, Styler};
use crate::text::{BlockText, BlockTextProjection, MorphologyError, RawText, TryFromText};
use crate::Render;

// TODO: Prevent nested annotations.
pub trait Annotate: Sized {
    fn annotate<A>(self, annotation: A) -> Annotated<Self, A>;

    fn attach<A>(self, annotation: A) -> Annotated<Self, Attachment<A>>;

    fn style<S>(self, style: S) -> Annotated<Self, Styler<S>>
    where
        S: Style;
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

    fn style<S>(self, style: S) -> Annotated<Self, Styler<S>>
    where
        S: Style,
    {
        self.annotate(Styler::from(style))
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
}

impl<T, S> Annotated<T, Styler<S>>
where
    S: Style,
{
    pub const fn styled(text: T, style: S) -> Self {
        Annotated {
            text,
            annotation: Styler::new(style),
        }
    }

    pub fn style(&self) -> &S {
        self.annotation.as_ref()
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
    type Mapped<U> = Annotated<U, A>
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

impl<T, A> Render for Annotated<T, Attachment<A>>
where
    T: Render,
{
    fn render(&self) -> Cow<str> {
        self.text.render()
    }

    fn render_into(&self, target: &mut impl Write) -> io::Result<()> {
        self.text.render_into(target)
    }
}

impl<T, S> Render for Annotated<T, Styler<S>>
where
    T: Render,
    S: Style,
{
    fn render(&self) -> Cow<str> {
        // TODO: ANSI style escape sequences cannot compose this way. For example, if the rendered
        //       `text` here is a line with colored segments and the `annotation` is a bold style,
        //       it will not be encoded properly and only some of the line will have the bold style
        //       when rendered by a terminal.
        //
        //       This will likely require a structural change that supports composing style
        //       elements.
        let text = self.text.render();
        self.annotation.encode(text.as_ref()).into_owned().into()
    }

    fn render_into(&self, target: &mut impl Write) -> io::Result<()> {
        self.annotation
            .encode_into(self.text.render().as_ref(), target)
    }
}
