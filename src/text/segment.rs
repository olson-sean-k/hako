use itertools::Itertools;
use std::borrow::Cow;
use std::fmt::Debug;
use std::io::{self, Write};
use std::marker::PhantomData;

use crate::cow::MoveCow;
use crate::text::layout::{AsBlockLayout, BlockLayout, LinearLayout};
use crate::text::morphology::{FlexKind, Grapheme, Morpheme, MorphemeFor, MorphemeKind};
use crate::text::{
    ops, BlockText, BlockTextProjection, Indexed, MorphologyError, RawText, StrExt as _,
    ToStringMut, TryFromText,
};
use crate::Render;

pub type SegmentFor<T, M> = Segment<<T as BlockTextProjection>::RawText, M>;

// TODO: Segments ignore non-ASCII line breaks (by design). Make sure this is documented.
// TODO: Consider `unicode-linebreak` or something similar if it seems that support for line
//       breaking Unicode control characters like LS and PS is justified.
// TODO: The derived implementations do not depend on the type parameter `M`. Implement them
//       explicitly to reflect this.
#[derive(Debug, Eq, Hash, PartialEq)]
pub struct Segment<T = String, M = FlexKind> {
    text: T,
    _phantom: PhantomData<fn() -> M>,
}

impl<T, M> Segment<T, M> {
    fn from_raw_text_unchecked(text: T) -> Self {
        Segment {
            text,
            _phantom: PhantomData,
        }
    }
}

impl<T, M> Segment<T, M>
where
    T: RawText,
    M: MorphemeKind,
{
    pub fn try_from_raw_text<U>(text: U) -> Result<Self, MorphologyError>
    where
        T: RawText + TryFrom<MoveCow<U>>,
        U: RawText,
    {
        Segment::try_from_text(text)
    }

    pub fn try_from_raw_text_or_joined<U>(text: U) -> Result<Self, MorphologyError>
    where
        T: TryFrom<MoveCow<String>> + TryFrom<MoveCow<U>>,
        U: RawText,
    {
        let mut lines = text.as_ref().split_at_ascii_line_breaks().peekable();
        let first = lines.next();
        if lines.peek().is_some() {
            Segment::try_from_text(first.into_iter().chain(lines).join(""))
        }
        else {
            drop(lines);
            Segment::try_from_text(text)
        }
    }

    pub fn try_from_raw_text_or_split<U>(text: U) -> Result<Vec<Self>, MorphologyError>
    where
        // TODO: This requires that split text is copied into a `String`, but may not if
        //       `split_at_ascii_line_breaks` were implemented by `U` and returned `MoveCow`
        //       instead.
        T: TryFrom<MoveCow<String>>,
        U: RawText,
    {
        text.as_ref()
            .split_at_ascii_line_breaks()
            .map(String::from)
            .map(Segment::try_from_text)
            .collect()
    }

    pub fn from_raw_text_or_empty<U>(text: U) -> Self
    where
        T: TryFrom<MoveCow<U>>,
        U: RawText,
    {
        match Segment::try_from_text(text) {
            Ok(text) => text,
            _ => Segment::empty(),
        }
    }

    pub fn assert<U>(text: U) -> Self
    where
        T: TryFrom<MoveCow<U>>,
        U: RawText,
    {
        Segment::try_from_text(text).expect("failed to construct block text")
    }

    pub const fn empty() -> Self {
        Segment {
            text: T::EMPTY,
            _phantom: PhantomData,
        }
    }

    pub fn try_map_raw_text<U, F>(self, f: F) -> Result<Segment<U, M>, MorphologyError>
    where
        U: RawText + TryFrom<MoveCow<U>>,
        F: FnOnce(T) -> U,
    {
        let Segment { text, .. } = self;
        Segment::try_from_text(f(text))
    }

    pub fn as_str(&self) -> &str {
        AsRef::<str>::as_ref(self)
    }

    // TODO: These bounds may be more specific than necessary: only the block bounds are needed
    //       here!
    // CLIPPY: This appears to be a false positive. An explicit lifetime is necessary for the GATs.
    #[allow(clippy::needless_lifetimes)]
    pub fn as_block_layout<'b>(
        &'b self,
    ) -> impl 'b + BlockLayout<Morpheme<'b> = MorphemeFor<'b, M>, Index = usize> {
        AsBlockLayout(self)
    }

    pub fn is_empty(&self) -> bool {
        self.text.as_ref().is_empty()
    }
}

impl<'t, M> Segment<Cow<'t, str>, M>
where
    M: MorphemeKind,
{
    pub fn into_owned(self) -> Segment<Cow<'static, str>, M> {
        let Segment { text, .. } = self;
        Segment::from_raw_text_unchecked(text.into_owned().into())
    }
}

impl<T, M> ops::Append for Segment<T, M>
where
    T: RawText + ToStringMut,
    M: MorphemeKind,
{
    fn append(mut self, rhs: Self) -> Self {
        self.text.to_string_mut().push_str(rhs.as_str());
        self
    }
}

impl<T, M> AsRef<str> for Segment<T, M>
where
    T: AsRef<str>,
{
    fn as_ref(&self) -> &str {
        self.text.as_ref()
    }
}

impl<T, M> BlockText for Segment<T, M>
where
    T: RawText,
    M: MorphemeKind,
{
    type RawText = T;
    type Morpheme<'t> = MorphemeFor<'t, M>
    where
        Self: 't;
    type Index = usize;

    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<Self::Index, Grapheme<'_>>> {
        self.text.as_ref().graphemes()
    }
}

impl<T, M> Clone for Segment<T, M>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Segment::from_raw_text_unchecked(self.text.clone())
    }
}

impl<T, M> Copy for Segment<T, M> where T: Copy {}

impl<T, M> ops::Extend for Segment<T, M>
where
    T: RawText + ToStringMut,
    M: MorphemeKind,
{
    fn extend<'t, I>(&'t mut self, morphemes: I) -> usize
    where
        I: IntoIterator<Item = Self::Morpheme<'t>>,
    {
        self.text
            .to_string_mut()
            .extend(morphemes.into_iter().map(Morpheme::into_string));
        self.width()
    }
}

impl<T, M> LinearLayout for Segment<T, M>
where
    T: RawText,
    M: MorphemeKind,
{
    fn width(&self) -> usize {
        self.text.as_ref().width()
    }
}

impl<T, M> Render for Segment<T, M>
where
    T: RawText,
    M: MorphemeKind,
{
    fn render(&self) -> Cow<str> {
        self.text.as_ref().into()
    }

    fn render_into(&self, target: &mut impl Write) -> io::Result<()> {
        target.write_all(self.text.as_ref().as_bytes())
    }
}

impl<T, M> ops::Truncate for Segment<T, M>
where
    T: RawText + ToStringMut,
    M: MorphemeKind,
{
    fn truncate(&mut self, max: usize) -> usize {
        let mut width = 0usize;
        // This iterator expression cannot use `find`, because it borrows the iterator, which
        // prevents the mutable borrow in the branch.
        if let Some(len) = self
            .text
            .as_ref()
            .graphemes()
            .skip_while(|grapheme| {
                width = width
                    .checked_add(grapheme.text.width())
                    .expect("overflow truncating text");
                width <= max
            })
            .take(1)
            .last()
            .map(|grapheme| grapheme.index)
        {
            self.text.to_string_mut().truncate(len);
        }
        self.width()
    }
}

impl<'t, M> TryFrom<Cow<'t, str>> for Segment<Cow<'t, str>, M>
where
    M: MorphemeKind,
{
    type Error = MorphologyError;

    fn try_from(text: Cow<'t, str>) -> Result<Self, Self::Error> {
        Segment::try_from_text(text)
    }
}

impl<'t, M> TryFrom<&'t str> for Segment<&'t str, M>
where
    M: MorphemeKind,
{
    type Error = MorphologyError;

    fn try_from(text: &'t str) -> Result<Self, Self::Error> {
        Segment::try_from_text(text)
    }
}

impl<M> TryFrom<String> for Segment<String, M>
where
    M: MorphemeKind,
{
    type Error = MorphologyError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Segment::try_from_text(text)
    }
}

impl<T, M> TryFromText<Segment<T, M>> for Segment<T, M> {
    fn try_from_text(segment: Segment<T, M>) -> Result<Self, MorphologyError> {
        Ok(segment)
    }
}

impl<T, U, M> TryFromText<U> for Segment<T, M>
where
    T: RawText + TryFrom<MoveCow<U>>,
    U: RawText,
    M: MorphemeKind,
{
    fn try_from_text(text: U) -> Result<Self, MorphologyError> {
        let text: T = text
            .strip_control_and_layout_points()
            .try_into()
            .map_err(|_| MorphologyError)?;
        if text
            .as_ref()
            .graphemes()
            .map(Indexed::into_text)
            .map(MorphemeFor::<M>::try_from)
            .all(|morpheme| morpheme.is_ok())
        {
            Ok(Segment::from_raw_text_unchecked(text))
        }
        else {
            Err(MorphologyError)
        }
    }
}
