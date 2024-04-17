use crate::text::Morpheme;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Layer<F, B> {
    Front(F),
    Back(B),
}

pub type MorphemeLayer<T> = Layer<T, T>;

// LHS is front and RHS is back.
pub trait Overlay<T> {
    type Morpheme: Morpheme;
    type Output;

    fn overlay<F>(self, rhs: T, f: F) -> Self::Output
    where
        F: FnMut(/* ... */) -> MorphemeLayer<Self::Morpheme>;
}

pub trait TryOverlay<T> {
}

impl<T, U> TryOverlay<U> for T where T: Overlay<U> {
}
