use std::borrow::Cow;
use std::fmt::Debug;
use std::num::NonZeroUsize;

use crate::text::modal::ModalWidth;
use crate::text::{self, MorphologyError, StrExt as _};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct Grapheme<'t> {
    text: Cow<'t, str>,
}

impl<'t> Grapheme<'t> {
    pub(in crate::text) const fn from_string_unchecked(text: Cow<'t, str>) -> Self {
        Grapheme { text }
    }

    pub fn from_point(point: char) -> Grapheme<'static> {
        Grapheme::from(point)
    }

    pub fn into_owned(self) -> Grapheme<'static> {
        let Grapheme { text } = self;
        Grapheme {
            text: text.into_owned().into(),
        }
    }

    pub fn into_string(self) -> Cow<'t, str> {
        self.text
    }

    pub fn points(&self) -> impl '_ + Iterator<Item = char> {
        self.text.chars()
    }

    pub fn width(&self) -> usize {
        self.text.as_ref().width()
    }
}

impl<'t> AsRef<str> for Grapheme<'t> {
    fn as_ref(&self) -> &str {
        self.text.as_ref()
    }
}

// No single Unicode code point encodes more than one grapheme.
impl<'t> From<char> for Grapheme<'t> {
    fn from(point: char) -> Self {
        Grapheme::from_string_unchecked(point.to_string().into())
    }
}

impl<'t> From<Flex<'t>> for Grapheme<'t> {
    fn from(morpheme: Flex<'t>) -> Self {
        match morpheme {
            Flex::Narrow(narrow) => narrow.into(),
            Flex::Wide(wide) => wide.into(),
        }
    }
}

impl<'t> From<Narrow<'t>> for Grapheme<'t> {
    fn from(narrow: Narrow<'t>) -> Self {
        narrow.grapheme
    }
}

impl<'t> From<Wide<'t>> for Grapheme<'t> {
    fn from(wide: Wide<'t>) -> Self {
        wide.grapheme
    }
}

impl<'t> TryFrom<Cow<'t, str>> for Grapheme<'t> {
    type Error = MorphologyError;

    fn try_from(text: Cow<'t, str>) -> Result<Self, Self::Error> {
        if text.as_ref().graphemes().take(2).count() == 1 {
            Ok(Grapheme::from_string_unchecked(text))
        }
        else {
            Err(MorphologyError)
        }
    }
}

impl<'t> TryFrom<&'t str> for Grapheme<'t> {
    type Error = MorphologyError;

    fn try_from(text: &'t str) -> Result<Self, Self::Error> {
        Cow::from(text).try_into()
    }
}

impl<'t> TryFrom<String> for Grapheme<'t> {
    type Error = MorphologyError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Cow::from(text).try_into()
    }
}

pub trait MorphemeKind: 'static {
    type Morpheme<'t>: Morpheme<'t>;
}

// TODO: Hmm, this is probably not useful.
// TODO: Use the term "blank" instead of "space" for morphemes. That is, abstract the notion of
//       non-display morphemes. "Space" is probably more appropriate for more general text types
//       though. See other TODOs about the terms "empty" and "blank" (remove "zero").
pub trait Blank: MorphemeKind {
    const BLANK: MorphemeFor<'static, Self>;
}

#[derive(Debug)]
pub enum FlexKind {}

impl MorphemeKind for FlexKind {
    type Morpheme<'t> = Flex<'t>;
}

#[derive(Debug)]
pub enum NarrowKind {}

impl MorphemeKind for NarrowKind {
    type Morpheme<'t> = Narrow<'t>;
}

#[derive(Debug)]
pub enum WideKind {}

impl MorphemeKind for WideKind {
    type Morpheme<'t> = Wide<'t>;
}

pub trait Morpheme<'t>: AsRef<str> + Into<Flex<'t>> + TryFrom<Grapheme<'t>> {
    type Kind: MorphemeKind<Morpheme<'t> = Self>;

    fn into_string(self) -> Cow<'t, str>;

    fn width(&self) -> NonZeroUsize;
}

pub type MorphemeFor<'t, M> = <M as MorphemeKind>::Morpheme<'t>;

pub type Flex<'t> = ModalWidth<Narrow<'t>, Wide<'t>>;

impl<'t> Flex<'t> {
    pub fn into_owned(self) -> Flex<'t> {
        match self {
            Flex::Narrow(narrow) => Flex::Narrow(narrow.into_owned()),
            Flex::Wide(wide) => Flex::Wide(wide.into_owned()),
        }
    }

    pub fn as_grapheme(&self) -> &Grapheme<'t> {
        match self {
            Flex::Narrow(ref narrow) => narrow.as_grapheme(),
            Flex::Wide(ref wide) => wide.as_grapheme(),
        }
    }
}

impl AsRef<str> for Flex<'_> {
    fn as_ref(&self) -> &str {
        match self {
            Flex::Narrow(ref narrow) => narrow.as_ref(),
            Flex::Wide(ref wide) => wide.as_ref(),
        }
    }
}

impl<'t> From<Narrow<'t>> for Flex<'t> {
    fn from(narrow: Narrow<'t>) -> Self {
        Flex::Narrow(narrow)
    }
}

impl<'t> From<Wide<'t>> for Flex<'t> {
    fn from(wide: Wide<'t>) -> Self {
        Flex::Wide(wide)
    }
}

impl<'t> Morpheme<'t> for Flex<'t> {
    type Kind = FlexKind;

    fn into_string(self) -> Cow<'t, str> {
        match self {
            Flex::Narrow(narrow) => narrow.into_string(),
            Flex::Wide(wide) => wide.into_string(),
        }
    }

    fn width(&self) -> NonZeroUsize {
        match self {
            Flex::Narrow(_) => Narrow::WIDTH,
            Flex::Wide(_) => Wide::WIDTH,
        }
    }
}

impl<'t> TryFrom<Grapheme<'t>> for Flex<'t> {
    type Error = MorphologyError;

    fn try_from(grapheme: Grapheme<'t>) -> Result<Self, Self::Error> {
        match NonZeroUsize::new(grapheme.width()) {
            Some(Narrow::WIDTH) => Ok(Narrow::from_grapheme_unchecked(grapheme).into()),
            Some(Wide::WIDTH) => Ok(Wide::from_grapheme_unchecked(grapheme).into()),
            _ => Err(MorphologyError),
        }
    }
}

// Narrow **or ambiguous one-column width**.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct Narrow<'t> {
    grapheme: Grapheme<'t>,
}

impl<'t> Narrow<'t> {
    pub const WIDTH: NonZeroUsize = NonZeroUsize::MIN;

    const fn from_grapheme_unchecked(grapheme: Grapheme<'t>) -> Self {
        Narrow { grapheme }
    }

    pub const fn space() -> Narrow<'static> {
        Narrow::from_grapheme_unchecked(Grapheme::from_string_unchecked(Cow::Borrowed(" ")))
    }

    pub fn into_owned(self) -> Narrow<'static> {
        let Narrow { grapheme } = self;
        Narrow {
            grapheme: grapheme.into_owned(),
        }
    }

    pub fn as_grapheme(&self) -> &Grapheme<'t> {
        &self.grapheme
    }

    pub fn is_ambiguous(&self) -> bool {
        // This function is defined here to avoid its use elsewhere: text width is non-CJK when
        // ambiguous.
        fn uax11_text_width_ambiguous_cjk(text: &str) -> usize {
            use unicode_width::UnicodeWidthStr;

            UnicodeWidthStr::width_cjk(text)
        }

        let text = self.as_ref();
        uax11_text_width_ambiguous_cjk(text) != text::uax11_text_width_ambiguous_non_cjk(text)
    }
}

impl<'t> AsRef<str> for Narrow<'t> {
    fn as_ref(&self) -> &str {
        self.grapheme.as_ref()
    }
}

impl<'t> Morpheme<'t> for Narrow<'t> {
    type Kind = NarrowKind;

    fn into_string(self) -> Cow<'t, str> {
        self.grapheme.into_string()
    }

    fn width(&self) -> NonZeroUsize {
        Self::WIDTH
    }
}

impl<'t> TryFrom<Grapheme<'t>> for Narrow<'t> {
    type Error = MorphologyError;

    fn try_from(grapheme: Grapheme<'t>) -> Result<Self, Self::Error> {
        if grapheme.width() == Narrow::WIDTH.get() {
            Ok(Narrow::from_grapheme_unchecked(grapheme))
        }
        else {
            Err(MorphologyError)
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct Wide<'t> {
    grapheme: Grapheme<'t>,
}

impl<'t> Wide<'t> {
    // SAFETY: The input `usize` is never zero (it is always the literal `2`).
    pub const WIDTH: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(2) };

    const fn from_grapheme_unchecked(grapheme: Grapheme<'t>) -> Self {
        Wide { grapheme }
    }

    pub const fn space() -> Wide<'static> {
        Wide::from_grapheme_unchecked(Grapheme::from_string_unchecked(Cow::Borrowed("　")))
    }

    pub fn into_owned(self) -> Wide<'static> {
        let Wide { grapheme } = self;
        Wide {
            grapheme: grapheme.into_owned(),
        }
    }

    pub fn as_grapheme(&self) -> &Grapheme<'t> {
        &self.grapheme
    }
}

impl<'t> AsRef<str> for Wide<'t> {
    fn as_ref(&self) -> &str {
        self.grapheme.as_ref()
    }
}

impl<'t> Morpheme<'t> for Wide<'t> {
    type Kind = WideKind;

    fn into_string(self) -> Cow<'t, str> {
        self.grapheme.into_string()
    }

    fn width(&self) -> NonZeroUsize {
        Self::WIDTH
    }
}

impl<'t> TryFrom<Grapheme<'t>> for Wide<'t> {
    type Error = MorphologyError;

    fn try_from(grapheme: Grapheme<'t>) -> Result<Self, Self::Error> {
        if grapheme.width() == Wide::WIDTH.get() {
            Ok(Wide::from_grapheme_unchecked(grapheme))
        }
        else {
            Err(MorphologyError)
        }
    }
}
