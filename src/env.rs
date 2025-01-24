use supports_color::ColorLevel;

use Stream::{Error, Output};

pub trait Query<T> {
    fn query(self) -> T;
}

impl<T> Query<T> for T {
    fn query(self) -> T {
        self
    }
}

// Types should implement this trait with the most specific `T` possible. Supersets of `T` can also
// be used with the correct `Query` implementations.
pub trait Detected<T> {
    fn detected(query: impl Query<T>) -> Self;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Stream {
    #[default]
    Error,
    Output,
}

impl From<Stream> for supports_color::Stream {
    fn from(stream: Stream) -> Self {
        match stream {
            Error => supports_color::Stream::Stderr,
            Output => supports_color::Stream::Stdout,
        }
    }
}

impl From<Stream> for supports_unicode::Stream {
    fn from(stream: Stream) -> Self {
        match stream {
            Error => supports_unicode::Stream::Stderr,
            Output => supports_unicode::Stream::Stdout,
        }
    }
}

impl Query<StyleEncoding> for Stream {
    fn query(self) -> StyleEncoding {
        // The color level reports no color support if the output stream is not a TTY.
        match supports_color::on(self.into()) {
            Some(ColorLevel { has_16m: true, .. }) => StyleEncoding::ANSI24,
            Some(ColorLevel { has_256: true, .. }) => StyleEncoding::ANSI8,
            Some(ColorLevel {
                has_basic: true, ..
            }) => StyleEncoding::ANSI4,
            _ => StyleEncoding(0),
        }
    }
}

impl Query<Configuration> for Stream {
    fn query(self) -> Configuration {
        Configuration {
            width: self.query(),
            encoding: self.query(),
        }
    }
}

impl Query<Encoding> for Stream {
    fn query(self) -> Encoding {
        Encoding {
            style: self.query(),
            text: self.query(),
        }
    }
}

impl Query<TextEncoding> for Stream {
    fn query(self) -> TextEncoding {
        // If the output stream is a TTY, then Unicode support is always reported.
        if supports_unicode::on(self.into()) {
            TextEncoding::UNICODE
        }
        else {
            // TODO: The lack of Unicode encoding does not necessarily mean that the output expects
            //       ASCII.
            TextEncoding::ASCII
        }
    }
}

impl Query<Width> for Stream {
    #[cfg(unix)]
    fn query(self) -> Width {
        use std::io;
        use std::os::fd::AsRawFd;

        let fd = match self {
            Error => io::stderr().as_raw_fd(),
            Output => io::stdout().as_raw_fd(),
        };
        Width(
            terminal_size::terminal_size_using_fd(fd)
                .map(|(width, _)| width.0.into())
                .unwrap_or(usize::MAX),
        )
    }

    #[cfg(not(unix))]
    fn query(self) -> Width {
        Width(
            terminal_size::terminal_size()
                .map(|(width, _)| width.0.into())
                .unwrap_or(usize::MAX),
        )
    }
}

// This information describes the detected configuration for the output stream and does **not**
// merely describe what features are supported by the device. For example, a TTY may support color,
// but a user may request no color output. In this case, `Configuration` and related types will
// reflect this and report no color in the configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Configuration {
    pub width: Width,
    pub encoding: Encoding,
}

impl Query<Encoding> for Configuration {
    fn query(self) -> Encoding {
        self.encoding
    }
}

impl Query<Width> for Configuration {
    fn query(self) -> Width {
        self.width
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Width(pub usize);

impl From<Width> for usize {
    fn from(width: Width) -> Self {
        width.0
    }
}

impl From<usize> for Width {
    fn from(width: usize) -> Self {
        Width(width)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Encoding {
    pub style: StyleEncoding,
    pub text: TextEncoding,
}

impl Query<StyleEncoding> for Encoding {
    fn query(self) -> StyleEncoding {
        self.style
    }
}

impl Query<TextEncoding> for Encoding {
    fn query(self) -> TextEncoding {
        self.text
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StyleEncoding(u8);

impl StyleEncoding {
    pub const NONE: Self = StyleEncoding(0b00000000);
    pub const ANSI4: Self = StyleEncoding(0b00000001);
    pub const ANSI8: Self = StyleEncoding(0b00000011);
    pub const ANSI24: Self = StyleEncoding(0b00000111);

    pub fn or(lhs: Self, rhs: Self) -> Self {
        StyleEncoding(lhs.0 | rhs.0)
    }

    pub fn is_in(&self, encoding: &Self) -> bool {
        (self.0 & encoding.0) == self.0
    }

    pub fn has_style_support(&self) -> bool {
        self.0 != 0
    }

    pub fn has_ansi4(&self) -> bool {
        self.0 & Self::ANSI4.0 == Self::ANSI4.0
    }

    pub fn has_ansi8(&self) -> bool {
        self.0 & Self::ANSI8.0 == Self::ANSI8.0
    }

    pub fn has_ansi24(&self) -> bool {
        self.0 & Self::ANSI24.0 == Self::ANSI24.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextEncoding(u8);

impl TextEncoding {
    pub const ASCII: Self = TextEncoding(0b00000001);
    pub const UNICODE: Self = TextEncoding(0b00000011);

    pub fn or(lhs: Self, rhs: Self) -> Self {
        TextEncoding(lhs.0 | rhs.0)
    }

    pub fn is_in(&self, encoding: &Self) -> bool {
        (self.0 & encoding.0) == self.0
    }

    pub fn has_ascii(&self) -> bool {
        self.0 & Self::ASCII.0 == Self::ASCII.0
    }

    pub fn has_unicode(&self) -> bool {
        self.0 & Self::UNICODE.0 == Self::UNICODE.0
    }
}
