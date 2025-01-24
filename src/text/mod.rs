mod line;
mod modal;
mod segment;

pub mod annotation;
pub mod geometry;
pub mod morphology;
pub mod ops;
pub mod render;
pub mod style;

use itertools::Itertools;
use std::borrow::Cow;
use std::convert::Infallible;
use std::fmt::Debug;
use std::iter;
use std::ops::{Bound, Range, RangeBounds};
use std::slice::SliceIndex;

use crate::cow::{IntoWritten, MoveCow};
use crate::env::TextEncoding;
use crate::text::morphology::{Grapheme, Morpheme, MorphemeKind};

pub use crate::text::line::{Line, LineIndex};
pub use crate::text::modal::ModalWidth;
pub use crate::text::segment::{BlankSegment, ContentSegment, Segment};

const CR: u8 = b'\r';
const LF: u8 = b'\n';

#[derive(Clone, Copy, Debug)]
pub enum BlockTextError {
    Control(ControlError),
    Morphology(MorphologyError),
}

impl From<ControlError> for BlockTextError {
    fn from(error: ControlError) -> Self {
        BlockTextError::Control(error)
    }
}

impl From<Infallible> for BlockTextError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

impl From<MorphologyError> for BlockTextError {
    fn from(error: MorphologyError) -> Self {
        BlockTextError::Morphology(error)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ControlError;

impl From<Infallible> for ControlError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MorphologyError;

impl From<Infallible> for MorphologyError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BoundaryError;

impl From<Infallible> for BoundaryError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

pub trait U8Ext: Copy {
    fn is_utf8_char_boundary(self) -> bool;
}

impl U8Ext for u8 {
    fn is_utf8_char_boundary(self) -> bool {
        (self as i8) >= -0x40
    }
}

pub trait StrExt {
    fn to_ascii_lossy(&self) -> Cow<'_, str>;

    fn split_at_ascii_line_breaks(&self) -> impl '_ + Iterator<Item = &'_ str>;

    fn lower_char_boundary(&self, index: usize) -> usize;

    fn upper_char_boundary(&self, index: usize) -> Option<usize>;

    fn get_or_truncate(&self, range: impl RangeBounds<usize>) -> &Self;

    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<usize, Grapheme<'_>>>;

    fn encoding(&self) -> TextEncoding;

    fn width(&self) -> usize;

    fn has_ascii_line_breaks(&self) -> bool;

    // Control and layout points are CC, CF, ZL, and ZP. These general categories affect the flow
    // and layout of text and the behavior of output targets like TTYs and printers.
    fn has_control_or_layout_points(&self) -> bool;
}

impl StrExt for str {
    fn to_ascii_lossy(&self) -> Cow<'_, str> {
        if self.is_ascii() {
            self.into()
        }
        else {
            let ascii: String = self
                .graphemes()
                .map(Indexed::into_text)
                .map(Grapheme::into_string)
                .map(|grapheme| -> Cow<'_, str> {
                    if grapheme.is_ascii() {
                        grapheme.into()
                    }
                    else {
                        // TODO: Allow this to be configured via a robust ASCII character type.
                        iter::repeat('?')
                            .take(grapheme.width())
                            .collect::<String>()
                            .into()
                    }
                })
                .collect();
            ascii.into()
        }
    }

    // Splits over unpaired CR (unlike `str::lines`). Discards line breaking control characters.
    // Exlcudes LS and PS, which are in the BMP but not ASCII. While CR and LF are the only line
    // breaking control characters in ASCII, this function conceptually splits over ASCII control
    // characters with **mandatory** Unicode line break properties.
    fn split_at_ascii_line_breaks(&self) -> impl '_ + Iterator<Item = &'_ str> {
        fn checkpoint(head: &mut usize, index: usize, n: usize) -> Range<usize> {
            let range = *head..index.saturating_sub(n.saturating_sub(1));
            *head = index
                .checked_add(1)
                .expect("overflow splitting text at ASCII line breaks");
            range
        }

        // This implementation depends on CR and LF never occuring as part of a plural code point
        // sequence in UTF-8. While this is true of CR and LF, it is **not true** for all BMP and
        // Unicode line breaking code points!
        let end = self.len();
        let mut head = 0;
        self.as_bytes()
            .iter()
            .copied()
            .enumerate()
            .peekable()
            .batching(move |bytes| {
                loop {
                    return match bytes.next() {
                        // Split over CR and CR LF sequences.
                        Some((index, CR)) => Some(match bytes.peek().copied() {
                            Some((index, LF)) => {
                                bytes.next();
                                checkpoint(&mut head, index, 2)
                            }
                            _ => checkpoint(&mut head, index, 1),
                        }),
                        // Split over LF.
                        Some((index, LF)) => Some(checkpoint(&mut head, index, 1)),
                        Some(_) => continue,
                        // Yield the remainder at EoT.
                        None => (head <= end).then(|| checkpoint(&mut head, end, 0)),
                    };
                }
            })
            .map(|range| self.get(range).expect("invalid UTF-8 slice"))
    }

    fn lower_char_boundary(&self, index: usize) -> usize {
        if index >= self.len() {
            self.len()
        }
        else {
            let lower = index.saturating_sub(3);
            lower
                + self.as_bytes()[lower..=index]
                    .iter()
                    .rposition(|byte| byte.is_utf8_char_boundary())
                    .unwrap()
        }
    }

    fn upper_char_boundary(&self, index: usize) -> Option<usize> {
        if index > self.len() {
            None
        }
        else {
            let upper = Ord::min(index + 4, self.len());
            Some(
                self.as_bytes()[index..upper]
                    .iter()
                    .position(|byte| byte.is_utf8_char_boundary())
                    .map_or(upper, |position| position + index),
            )
        }
    }

    fn get_or_truncate(&self, range: impl RangeBounds<usize>) -> &Self {
        use Bound::{Excluded, Included, Unbounded};

        // TODO: This code assumes that the start and end of the range are in ascending order (that
        //       is, start is less than end). If a reversed range is given, then the boundary
        //       search proceeds in the opposite direction w.r.t. bounded terminals.
        let start = self.lower_char_boundary(match range.start_bound() {
            Excluded(start) => start.saturating_add(1),
            Included(start) => *start,
            Unbounded => 0,
        });
        let end = match self.upper_char_boundary(match range.end_bound() {
            Excluded(end) => *end,
            Included(end) => end.saturating_add(1),
            Unbounded => self.len(),
        }) {
            Some(end) => end,
            _ => self.len(),
        };
        self.get(start..end).unwrap()
    }

    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<usize, Grapheme<'_>>> {
        self::uax29_text_grapheme_indices(self)
    }

    fn encoding(&self) -> TextEncoding {
        if self.is_ascii() {
            TextEncoding::ASCII
        }
        else {
            TextEncoding::UNICODE
        }
    }

    fn width(&self) -> usize {
        self::uax11_text_width_ambiguous_non_cjk(self)
    }

    fn has_ascii_line_breaks(&self) -> bool {
        // Detect any and all occurences of CR and LF. Note that both CR and LF are considered a
        // line break even when not adjacent to another line breaking control character (i.e., a
        // lone CR).
        self.as_bytes().iter().copied().any(ucs_ascii_is_cr_lf)
    }

    fn has_control_or_layout_points(&self) -> bool {
        self.chars().any(self::uax44_point_is_cc_cf_zl_zp)
    }
}

// Though this trait has the shape of an `AsMut` conversion, it may convert `self` prior to
// returning its reference, so it uses "to" nomenclature rather than "as".
pub trait ToStringMut {
    fn to_string_mut(&mut self) -> &mut String;
}

impl<'t> ToStringMut for Cow<'t, str> {
    fn to_string_mut(&mut self) -> &mut String {
        self.to_mut()
    }
}

impl ToStringMut for String {
    fn to_string_mut(&mut self) -> &mut String {
        self
    }
}

pub trait Strip: IntoWritten + Sized {
    // Like `str::replace`, but strictly removes and does not copy when the `Pattern` matches
    // nothing.
    // TODO: Implement this in terms of `std::str::Pattern` when it is stabilized.
    fn strip<F>(self, pattern: F) -> MoveCow<Self>
    where
        F: FnMut(char) -> bool;

    fn strip_control_and_layout_points(self) -> MoveCow<Self> {
        self.strip(self::uax44_point_is_cc_cf_zl_zp)
    }
}

impl<T> Strip for T
where
    T: AsRef<str> + IntoWritten,
    T::Written: From<String>,
{
    // TODO: Oh man, test this.
    fn strip<F>(self, pattern: F) -> MoveCow<Self>
    where
        F: FnMut(char) -> bool,
    {
        fn checkpoint(head: &mut usize, index: usize, matched: &str) -> Range<usize> {
            let range = *head..index;
            *head = index
                .checked_add(matched.len())
                .expect("overflow stripping text");
            range
        }

        fn push(stripped: &mut String, haystack: &str, index: impl SliceIndex<str, Output = str>) {
            stripped.push_str(haystack.get(index).expect("invalid UTF-8 slice"));
        }

        let haystack = self.as_ref();
        let mut matches = haystack.match_indices(pattern);
        let mut head = 0usize;
        if let Some((index, matched)) = matches.next() {
            let mut stripped = String::with_capacity(haystack.len());
            let mut checkpoint_and_push = |index, matched| {
                push(
                    &mut stripped,
                    haystack,
                    checkpoint(&mut head, index, matched),
                );
            };
            checkpoint_and_push(index, matched);
            for (index, matched) in matches {
                checkpoint_and_push(index, matched);
            }
            push(&mut stripped, haystack, head..);
            MoveCow::Written(stripped.into())
        }
        else {
            MoveCow::Unwritten(self)
        }
    }
}

pub trait RawText: AsRef<str> + IntoWritten + Strip {
    const EMPTY: Self;
}

impl<'t> RawText for Cow<'t, str> {
    const EMPTY: Self = Cow::Borrowed("");
}

impl<'t> RawText for &'t str {
    const EMPTY: Self = "";
}

impl RawText for String {
    const EMPTY: Self = String::new();
}

impl<'t> RawText for &'t String {
    const EMPTY: Self = &String::new();
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Indexed<N, T> {
    pub index: N,
    pub text: T,
}

impl<N, T> Indexed<N, T> {
    pub fn into_text(self) -> T {
        self.text
    }

    pub fn map_index<U, F>(self, f: F) -> Indexed<U, T>
    where
        F: FnOnce(N) -> U,
    {
        let Indexed { index, text } = self;
        Indexed {
            index: f(index),
            text,
        }
    }

    pub fn map_text<U, F>(self, f: F) -> Indexed<N, U>
    where
        F: FnOnce(T) -> U,
    {
        let Indexed { index, text } = self;
        Indexed {
            index,
            text: f(text),
        }
    }

    pub fn as_ref(&self) -> Indexed<N, &T>
    where
        N: Clone,
    {
        Indexed {
            index: self.index.clone(),
            text: &self.text,
        }
    }

    pub fn as_mut(&mut self) -> Indexed<N, &mut T>
    where
        N: Clone,
    {
        Indexed {
            index: self.index.clone(),
            text: &mut self.text,
        }
    }
}

impl<N, T> Indexed<N, Option<T>> {
    pub fn transpose(self) -> Option<Indexed<N, T>> {
        let Indexed { index, text } = self;
        text.map(|text| Indexed { index, text })
    }
}

impl<N, T, E> Indexed<N, Result<T, E>> {
    pub fn transpose(self) -> Result<Indexed<N, T>, E> {
        let Indexed { index, text } = self;
        text.map(|text| Indexed { index, text })
    }
}

impl<T> Indexed<usize, T> {
    pub fn zero(text: T) -> Self {
        Indexed { index: 0, text }
    }
}

impl<N, T> From<(N, T)> for Indexed<N, T> {
    fn from((index, text): (N, T)) -> Self {
        Indexed { index, text }
    }
}

// Some types have infallible identity implementations of this conversion trait. This trait is not
// paired with a `FromText` trait with an infallible blanket implementation, because this prevents
// some important general implementations of `TryFromText`.
pub trait TryFromText<T>: Sized {
    type Error;

    fn try_from_text(text: T) -> Result<Self, Self::Error>;

    fn assert(text: T) -> Self
    where
        Self::Error: Debug,
    {
        Self::try_from_text(text).expect("failed to construct block text")
    }
}

pub trait TryIntoText<T>: Sized
where
    T: TryFromText<Self>,
{
    fn try_into_text(self) -> Result<T, T::Error> {
        T::try_from_text(self)
    }
}

impl<T, U> TryIntoText<U> for T where U: TryFromText<T> {}

pub trait BlockText:
    BlockTextProjection<RawText = <Self as BlockText>::RawText, BlockText = Self>
{
    type RawText: RawText;
    type Morpheme<'t>: Morpheme<'t>
    where
        Self: 't;
    type Index: Eq;

    fn graphemes(&self) -> impl '_ + Clone + Iterator<Item = Indexed<Self::Index, Grapheme<'_>>>;

    // TODO: `BlockText` types must never allow construction from text containing non-morphemes.
    //       Ideally then, this function need not examine the text and can instead construct
    //       morphemes via `unchecked`! In a default implementation though, shenanigans are
    //       possible, especially if downstream code implements this trait (which would implicitly
    //       grant access to `unchecked`, which is intentionally not in the public API). This trait
    //       could be sealed or an internal function could be used to trivially implement this
    //       function instead of providing a default.
    fn morphemes(
        &self,
    ) -> impl '_ + Clone + Iterator<Item = Indexed<Self::Index, Self::Morpheme<'_>>> {
        self.graphemes()
            .map(|indexed| indexed.map_text(Self::Morpheme::try_from).transpose())
            .map(move |morpheme| match morpheme {
                Ok(morpheme) => morpheme,
                _ => panic!("non-morpheme in text"),
            })
    }
}

pub trait BlockTextProjection {
    type RawText: RawText;
    type BlockText: BlockText<RawText = Self::RawText>;
    type Mapped<T>: BlockTextProjection
    where
        T: BlockText;

    fn into_block_text(self) -> Self::BlockText;

    fn map_block_text<T, F>(self, f: F) -> Self::Mapped<T>
    where
        T: BlockText,
        F: FnOnce(Self::BlockText) -> T;

    fn as_block_text(&self) -> &Self::BlockText;

    fn as_block_text_mut(&mut self) -> &mut Self::BlockText;
}

impl<T> BlockTextProjection for T
where
    T: BlockText,
{
    type RawText = <T as BlockText>::RawText;
    type BlockText = T;
    type Mapped<U>
        = U
    where
        U: BlockText;

    fn into_block_text(self) -> Self::BlockText {
        self
    }

    // TODO: At time of writing, `rustc` claims that `U` and `Self::Mapped<U>` are incompatible
    //       (not the same type), despite the definition `type Mapped<U> = U;`. Because these types
    //       are always the same in this implementation, the output `U` of `F` is transmuted into
    //       `Self::Mapped<U>`.
    fn map_block_text<U, F>(self, f: F) -> Self::Mapped<U>
    where
        U: BlockText,
        F: FnOnce(Self::BlockText) -> U,
    {
        use std::mem;

        // SAFETY: The types `U` and `Self::Mapped<U>` must be the same or this transmutation is
        //         very likely UB and this API is unsound.
        unsafe { mem::transmute_copy::<U, Self::Mapped<U>>(&f(self)) }
    }

    fn as_block_text(&self) -> &Self::BlockText {
        self
    }

    fn as_block_text_mut(&mut self) -> &mut Self::BlockText {
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BlankText(pub usize);

impl BlankText {
    pub const fn empty() -> Self {
        BlankText(0)
    }

    pub fn from_min_width<M>(width: usize) -> Self
    where
        M: MorphemeKind,
    {
        BlankText(
            width
                .checked_add(width % M::MIN_WIDTH.get())
                .expect("overflow determining width of blank text"),
        )
    }

    pub fn from_max_width<M>(width: usize) -> Self
    where
        M: MorphemeKind,
    {
        BlankText(width.saturating_sub(width % M::MIN_WIDTH.get()))
    }

    pub fn from_min_width_morpheme_count<M>(n: usize) -> Self
    where
        M: MorphemeKind,
    {
        BlankText(
            n.checked_mul(M::MIN_WIDTH.get())
                .expect("overflow determining width of blank text"),
        )
    }

    pub fn width(&self) -> usize {
        self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

impl From<BlankText> for usize {
    fn from(text: BlankText) -> Self {
        text.0
    }
}

pub(self) fn ucs_ascii_is_cr_lf(byte: u8) -> bool {
    matches!(byte, CR | LF)
}

pub(self) fn uax44_point_is_cc_cf_zl_zp(point: char) -> bool {
    use unicode_properties::{GeneralCategory, UnicodeGeneralCategory};

    use GeneralCategory::{Control, Format, LineSeparator, ParagraphSeparator};

    matches!(
        point.general_category(),
        Control | Format | LineSeparator | ParagraphSeparator
    )
}

// Here, "ambiguous non-CJK" means that UAX11 ambiguous graphemes are assigned the "non-CJK" column
// width of one (rather than two, which is typically more compatible in CJK contexts). Generally,
// ambiguous and halfwidth graphemes are both mapped to narrow morphemes and are treated the same.
pub(self) fn uax11_text_width_ambiguous_non_cjk(text: &str) -> usize {
    use unicode_width::UnicodeWidthStr;

    // This considers some potentially troublesome ASCII whitespace characters as zero-width, which
    // works well here! For example, TAB is zero-width and so is **not** a morpheme.
    UnicodeWidthStr::width(text)
}

pub(self) fn uax29_text_grapheme_indices(
    text: &str,
) -> impl '_ + Clone + Iterator<Item = Indexed<usize, Grapheme<'_>>> {
    use unicode_segmentation::UnicodeSegmentation;

    UnicodeSegmentation::grapheme_indices(text, true)
        .map(Indexed::from)
        .map(|grapheme| {
            grapheme
                .map_text(Cow::from)
                .map_text(Grapheme::from_string_unchecked)
        })
}

#[cfg(test)]
mod tests {
    use crate::text::annotation::AnnotatedText;
    use crate::text::{ContentSegment, Line, Segment, StrExt as _};

    #[test]
    fn split_at_ascii_line_breaks() {
        fn lines(text: &str) -> Vec<&str> {
            text.split_at_ascii_line_breaks().collect()
        }

        assert_eq!(lines(""), vec![""]);
        assert_eq!(lines("\n"), vec!["", ""]);
        assert_eq!(lines("a\nb"), vec!["a", "b"]);
        assert_eq!(lines("a\r\nb"), vec!["a", "b"]);
        assert_eq!(lines("a\r\r\nb"), vec!["a", "", "b"]);
        assert_eq!(lines("a\n\rb"), vec!["a", "", "b"]);
        assert_eq!(lines("a\r\r\n\r\nb"), vec!["a", "", "", "b"]);
        assert_eq!(lines("\na"), vec!["", "a"]);
        assert_eq!(lines("\n\na"), vec!["", "", "a"]);
        assert_eq!(lines("a\n"), vec!["a", ""]);
        assert_eq!(lines("a\n\n"), vec!["a", "", ""]);
    }

    #[test]
    fn line_from_split_text() {
        let lines = Line::<Segment>::try_from_split_raw_text("text\ntext").unwrap();
        assert_eq!(lines.len(), 2);
        for line in lines {
            assert_eq!(line.to_string(), "text");
        }
    }

    #[test]
    fn render_block_text() {
        let segment: Segment = ContentSegment::try_from_raw_text("text").unwrap().into();
        assert_eq!(segment.display().default().to_string(), "text");
        let annotated = AnnotatedText::attached(segment, 0usize);
        assert_eq!(annotated.display::<()>().default().to_string(), "text");
        let line: Line<_> = [annotated.clone(), annotated].into_iter().collect();
        assert_eq!(line.display::<()>().default().to_string(), "texttext\n");
    }

    // TODO: Assert that the ANSI8 escape codes are present and correct in the rendered text.
    #[cfg(feature = "owo-colors")]
    #[test]
    fn render_styled_block_text() {
        use owo_colors::Style;

        use crate::env::Stream;
        use crate::text::annotation::Annotate;
        use crate::text::{BlankText, TryFromText};

        let red = Style::new().red();
        let green = Style::new().green().blink();
        let blue = Style::new().blue();
        let bold = Style::new().bold();

        let line = Line::from_iter([
            Segment::<&str>::assert("red").style(red),
            Segment::assert(BlankText(3)).into(),
            Segment::assert("green").style(green),
            Segment::assert(BlankText(3)).into(),
            Segment::assert("blue").style(blue),
        ])
        .style(bold);
        eprint!("{}", line.display().detected(Stream::Error));
    }
}
