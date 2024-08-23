use std::borrow::Cow;

pub trait IntoWritten {
    type Written;

    fn into_written(self) -> Self::Written;
}

impl<'t> IntoWritten for Cow<'t, str> {
    type Written = String;

    fn into_written(self) -> Self::Written {
        self.into_owned()
    }
}

impl IntoWritten for String {
    type Written = Self;

    fn into_written(self) -> Self::Written {
        self
    }
}

impl<'t> IntoWritten for &'t String {
    type Written = String;

    fn into_written(self) -> Self::Written {
        self.clone()
    }
}

impl<'t> IntoWritten for &'t str {
    type Written = String;

    fn into_written(self) -> Self::Written {
        self.into()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MoveCow<T>
where
    T: IntoWritten,
{
    Unwritten(T),
    Written(T::Written),
}

impl<'t, T> From<MoveCow<T>> for Cow<'t, str>
where
    Self: From<T> + From<T::Written>,
    T: IntoWritten,
{
    fn from(text: MoveCow<T>) -> Self {
        match text {
            MoveCow::Unwritten(text) => text.into(),
            MoveCow::Written(text) => text.into(),
        }
    }
}

impl<T> From<MoveCow<T>> for String
where
    Self: From<T> + From<T::Written>,
    T: IntoWritten,
{
    fn from(text: MoveCow<T>) -> Self {
        match text {
            MoveCow::Unwritten(text) => text.into(),
            MoveCow::Written(text) => text.into(),
        }
    }
}

impl<'t> TryFrom<MoveCow<&'t str>> for &'t str {
    type Error = ();

    fn try_from(text: MoveCow<&'t str>) -> Result<Self, Self::Error> {
        match text {
            MoveCow::Unwritten(text) => Ok(text),
            _ => Err(()),
        }
    }
}
