use itertools::{Itertools as _, Position};
use std::borrow::Cow;
use std::fmt::Debug;
use std::io::{self, Write};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr as UnicodeWidth;

use crate::Render;

pub(crate) trait ContentSlice<C>
where
    C: Content,
{
    fn width(&self) -> usize;
}

impl<C> ContentSlice<C> for [C]
where
    C: Content,
{
    fn width(&self) -> usize {
        self.iter()
            .map(|content| content.width())
            .max()
            .unwrap_or(0)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Grapheme<'t>(Cow<'t, str>);

impl<'t> Grapheme<'t> {
    pub const SPACE: Grapheme<'static> = Grapheme(Cow::Borrowed(" "));

    fn unchecked(text: &'t str) -> Self {
        Grapheme(text.into())
    }

    pub fn get(&self) -> &str {
        self.0.as_ref()
    }

    pub fn code_points(&self) -> impl '_ + Iterator<Item = char> {
        self.0.chars()
    }
}

impl<'t> AsRef<str> for Grapheme<'t> {
    fn as_ref(&self) -> &str {
        self.get()
    }
}

impl From<char> for Grapheme<'static> {
    fn from(point: char) -> Self {
        Grapheme(String::from(point).into())
    }
}

impl<'t> TryFrom<&'t str> for Grapheme<'t> {
    type Error = ();

    fn try_from(text: &'t str) -> Result<Self, Self::Error> {
        if text.graphemes(true).take(2).count() == 1 {
            Ok(Grapheme(text.into()))
        }
        else {
            Err(())
        }
    }
}

pub struct Congruent<C>
where
    C: Content,
{
    left: C,
    right: C,
}

impl<C> Congruent<C>
where
    C: Content,
{
    pub fn into_left_right(self) -> (C, C) {
        self.into()
    }

    pub fn left(&self) -> &C {
        &self.left
    }

    pub fn right(&self) -> &C {
        &self.right
    }
}

impl<C> From<Congruent<C>> for (C, C)
where
    C: Content,
{
    fn from(congruent: Congruent<C>) -> Self {
        let Congruent { left, right } = congruent;
        (left, right)
    }
}

impl<C> TryFrom<(C, C)> for Congruent<C>
where
    C: Content,
{
    type Error = ();

    fn try_from((left, right): (C, C)) -> Result<Self, Self::Error> {
        (left.width() == right.width())
            .then(|| Congruent { left, right })
            .ok_or(())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(usize)]
pub enum Layer<T = ()> {
    Front(T),
    Back(T),
}

pub trait Content: Clone + Debug + Sized + Render {
    fn empty() -> Self;

    fn grapheme(glyph: Grapheme) -> Self;

    fn space() -> Self {
        Self::grapheme(Grapheme::SPACE)
    }

    #[must_use]
    fn repeat(self, n: usize) -> Self;

    #[must_use]
    fn truncate(self, width: usize) -> Self;

    fn into_lines(self) -> Vec<Self>;

    #[must_use]
    fn concatenate(left: Self, right: Self) -> Self;

    #[rustfmt::skip]
    fn overlay_with(
        content: Congruent<Self>,
        f: impl FnMut(&Grapheme, &Grapheme) -> Layer,
    ) -> Self;

    fn width(&self) -> usize;
}

impl<'t> Content for Cow<'t, str> {
    fn empty() -> Self {
        "".into()
    }

    fn grapheme(glyph: Grapheme) -> Self {
        glyph.get().to_owned().into()
    }

    fn space() -> Self {
        Grapheme::SPACE.0.clone()
    }

    fn repeat(self, n: usize) -> Self {
        self.as_ref().repeat(n).into()
    }

    fn truncate(self, width: usize) -> Self {
        self.graphemes(true)
            .take(width)
            .fold(String::new(), |mut output, glyph| {
                output.push_str(glyph);
                output
            })
            .into()
    }

    fn into_lines(self) -> Vec<Self> {
        self.lines()
            .map(From::from)
            .map(Cow::into_owned)
            .map(From::from)
            .collect()
    }

    fn concatenate(left: Self, right: Self) -> Self {
        format!("{}{}", left, right).into()
    }

    fn overlay_with(
        content: Congruent<Self>,
        mut f: impl FnMut(&Grapheme, &Grapheme) -> Layer,
    ) -> Self {
        let (front, back) = content.into();
        front
            .graphemes(true)
            .zip(back.graphemes(true))
            .map(
                |(front, back)| match f(&Grapheme::unchecked(front), &Grapheme::unchecked(back)) {
                    Layer::Front(_) => front,
                    Layer::Back(_) => back,
                },
            )
            .collect()
    }

    fn width(&self) -> usize {
        <str as UnicodeWidth>::width(self)
    }
}

impl Content for String {
    fn empty() -> Self {
        String::new()
    }

    fn grapheme(glyph: Grapheme) -> Self {
        String::from(glyph.get())
    }

    fn repeat(self, n: usize) -> Self {
        str::repeat(&self, n)
    }

    fn truncate(self, width: usize) -> Self {
        self.graphemes(true)
            .take(width)
            .fold(String::new(), |mut output, glyph| {
                output.push_str(glyph);
                output
            })
    }

    fn into_lines(self) -> Vec<Self> {
        self.lines().map(From::from).collect()
    }

    fn concatenate(left: Self, right: Self) -> Self {
        format!("{}{}", left, right)
    }

    fn overlay_with(
        content: Congruent<Self>,
        mut f: impl FnMut(&Grapheme, &Grapheme) -> Layer,
    ) -> Self {
        let (front, back) = content.into();
        front
            .graphemes(true)
            .zip(back.graphemes(true))
            .map(
                |(front, back)| match f(&Grapheme::unchecked(front), &Grapheme::unchecked(back)) {
                    Layer::Front(_) => front,
                    Layer::Back(_) => back,
                },
            )
            .collect()
    }

    fn width(&self) -> usize {
        <str as UnicodeWidth>::width(self)
    }
}

pub trait Style: Clone + Debug {
    fn apply<'t>(&self, text: &'t str) -> Cow<'t, str>;
}

impl Style for () {
    fn apply<'t>(&self, text: &'t str) -> Cow<'t, str> {
        text.into()
    }
}

// TODO: Consider using `Option<S>` instead of requiring `Default`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Styled<C = String, S = ()>
where
    C: AsRef<str> + Content + From<String>,
    S: Style,
{
    fragments: Vec<(S, C)>,
}

impl<C, S> Styled<C, S>
where
    C: AsRef<str> + Content + From<String>,
    S: Style,
{
    pub fn new(style: S, content: impl Into<C>) -> Self {
        Styled {
            fragments: vec![(style, content.into())],
        }
    }

    #[must_use]
    pub fn restyle(self, style: S) -> Self {
        Styled {
            fragments: vec![(
                style,
                self.fragments
                    .into_iter()
                    .map(|(_, content)| content)
                    .fold(C::empty(), |output, content| {
                        C::concatenate(output, content)
                    }),
            )],
        }
    }

    fn fragment_indexed_graphemes<'i>(&'i self) -> impl 'i + Iterator<Item = (usize, Grapheme)> {
        self.fragments
            .iter()
            .enumerate()
            .flat_map(|(index, (_, content))| {
                content
                    .as_ref()
                    .graphemes(true)
                    .map(move |point| (index, Grapheme::unchecked(point)))
            })
    }
}

impl<'t, S> Styled<Cow<'t, str>, S>
where
    S: Style,
{
    pub fn into_owned(self) -> Styled<Cow<'static, str>, S> {
        let Styled { fragments } = self;
        Styled {
            fragments: fragments
                .into_iter()
                .map(|(style, content)| (style, content.into_owned().into()))
                .collect(),
        }
    }
}

impl<C, S> Content for Styled<C, S>
where
    C: AsRef<str> + Content + From<String>,
    S: Default + Style,
{
    fn empty() -> Self {
        Styled { fragments: vec![] }
    }

    fn grapheme(glyph: Grapheme) -> Self {
        Styled {
            fragments: vec![(S::default(), C::grapheme(glyph))],
        }
    }

    fn repeat(self, n: usize) -> Self {
        let m = self.fragments.len();
        Styled {
            fragments: self
                .fragments
                .into_iter()
                .enumerate()
                .cycle()
                .take(m * n)
                .coalesce(|(i, previous), (j, next)| {
                    if i == j {
                        Ok((0, (previous.0, Content::concatenate(previous.1, next.1))))
                    }
                    else {
                        Err(((i, previous), (j, next)))
                    }
                })
                .map(|(_, content)| content)
                .collect(),
        }
    }

    fn truncate(self, width: usize) -> Self {
        let mut sum = 0usize;
        let mut fragments: Vec<_> = self
            .fragments
            .into_iter()
            .take_while(|(_, content)| {
                let has_capacity = sum < width;
                sum += content.width();
                has_capacity
            })
            .collect();
        let n = sum.saturating_sub(width);
        if n > 0 {
            if let Some(fragment) = fragments.pop().map(|(style, content)| {
                let width = content.width() - n;
                (style, content.truncate(width))
            }) {
                fragments.push(fragment);
            }
        }
        Styled { fragments }
    }

    fn into_lines(self) -> Vec<Self> {
        let mut lines = vec![];
        let mut line = Styled::empty();
        for (style, content) in self.fragments {
            for split in content.as_ref().lines().with_position() {
                match split {
                    Position::Only(split) | Position::First(split) => {
                        line = Content::concatenate(
                            line,
                            Styled::new(style.clone(), split.to_owned()),
                        );
                    }
                    Position::Middle(split) | Position::Last(split) => {
                        lines.push(line);
                        line = Styled::new(style.clone(), split.to_owned());
                    }
                }
            }
        }
        lines.push(line);
        lines
    }

    fn concatenate(mut left: Self, mut right: Self) -> Self {
        left.fragments.append(&mut right.fragments);
        Styled {
            fragments: left.fragments,
        }
    }

    fn overlay_with(
        content: Congruent<Self>,
        mut f: impl FnMut(&Grapheme, &Grapheme) -> Layer,
    ) -> Self {
        let (front, back) = content.into();
        let overlay = front
            .fragment_indexed_graphemes()
            .zip(back.fragment_indexed_graphemes())
            .map(|((i, front), (j, back))| match f(&front, &back) {
                Layer::Front(_) => (Layer::Front(i), front),
                Layer::Back(_) => (Layer::Back(j), back),
            })
            .group_by(|(index, _)| *index)
            .into_iter()
            .fold(Styled::empty(), |output, (index, group)| {
                let text: String = group
                    .into_iter()
                    .map(|(_, glyph)| glyph.get().to_owned())
                    .collect();
                let style = match index {
                    Layer::Front(index) => front.fragments.get(index).unwrap().0.clone(),
                    Layer::Back(index) => back.fragments.get(index).unwrap().0.clone(),
                };
                Content::concatenate(output, Styled::new(style, text))
            });
        overlay
    }

    fn width(&self) -> usize {
        self.fragments
            .iter()
            .map(|(_, content)| content.as_ref().width())
            .sum()
    }
}

impl<C, S> Render for Styled<C, S>
where
    C: AsRef<str> + Content + From<String>,
    S: Style,
{
    fn render_into(&self, target: &mut impl Write) -> io::Result<()> {
        for (style, content) in self.fragments.iter() {
            target.write_all(style.apply(content.as_ref()).as_bytes())?;
        }
        Ok(())
    }

    fn render(&self) -> Cow<str> {
        self.fragments
            .iter()
            .fold(String::new(), |mut output, (style, content)| {
                output.push_str(style.apply(content.as_ref()).as_ref());
                output
            })
            .into()
    }
}
