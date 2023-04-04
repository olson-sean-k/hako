mod decoder;

pub mod typed;
pub mod valued;

pub trait HorizontalEnvelope<T>: Sized {
    fn left(&self) -> &T;

    fn right(&self) -> &T;

    fn horizontally_aligned_at<H>(&self) -> &T
    where
        H: typed::HorizontalAlignment,
    {
        H::aligned(self)
    }
}

pub trait VerticalEnvelope<T>: Sized {
    fn top(&self) -> &T;

    fn bottom(&self) -> &T;

    fn vertically_aligned_at<V>(&self) -> &T
    where
        V: typed::VerticalAlignment,
    {
        V::aligned(self)
    }
}

pub trait AxialEnvelope<T>: Sized {
    fn horizontal(&self) -> &T;

    fn vertical(&self) -> &T;

    fn axially_aligned_at<A>(&self) -> &T
    where
        A: typed::Axis,
    {
        A::aligned(self)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Horizontal<T> {
    pub left: T,
    pub right: T,
}

impl<T> Horizontal<T> {
    pub fn aligned(&self, alignment: valued::HorizontalAlignment) -> &T {
        match alignment {
            valued::HorizontalAlignment::Left => &self.left,
            valued::HorizontalAlignment::Right => &self.right,
        }
    }

    pub fn fold_horizontally_at<H, U, F>(&self, mut f: F) -> U
    where
        H: typed::HorizontalAlignment,
        H::Opposite: typed::HorizontalAlignment,
        F: FnMut(&T, &T) -> U,
    {
        f(
            self.horizontally_aligned_at::<H>(),
            self.horizontally_aligned_at::<H::Opposite>(),
        )
    }
}

impl<T> Horizontal<Vertical<T>> {
    pub fn transpose(self) -> Vertical<Horizontal<T>> {
        let Horizontal { left, right } = self;
        Vertical {
            top: Horizontal {
                left: left.top,
                right: right.top,
            },
            bottom: Horizontal {
                left: left.bottom,
                right: right.bottom,
            },
        }
    }
}

impl<T> HorizontalEnvelope<T> for Horizontal<T> {
    fn left(&self) -> &T {
        &self.left
    }

    fn right(&self) -> &T {
        &self.right
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Vertical<T> {
    pub top: T,
    pub bottom: T,
}

impl<T> Vertical<T> {
    pub fn aligned(&self, alignment: valued::VerticalAlignment) -> &T {
        match alignment {
            valued::VerticalAlignment::Top => &self.top,
            valued::VerticalAlignment::Bottom => &self.bottom,
        }
    }

    pub fn fold_vertically_at<H, U, F>(&self, mut f: F) -> U
    where
        H: typed::VerticalAlignment,
        H::Opposite: typed::VerticalAlignment,
        F: FnMut(&T, &T) -> U,
    {
        f(
            self.vertically_aligned_at::<H>(),
            self.vertically_aligned_at::<H::Opposite>(),
        )
    }
}

impl<T> Vertical<Horizontal<T>> {
    pub fn transpose(self) -> Horizontal<Vertical<T>> {
        let Vertical { top, bottom } = self;
        Horizontal {
            left: Vertical {
                top: top.left,
                bottom: bottom.left,
            },
            right: Vertical {
                top: top.right,
                bottom: bottom.right,
            },
        }
    }
}

impl<T> VerticalEnvelope<T> for Vertical<T> {
    fn top(&self) -> &T {
        &self.top
    }

    fn bottom(&self) -> &T {
        &self.bottom
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Square<T> {
    pub left: T,
    pub right: T,
    pub top: T,
    pub bottom: T,
}

impl<T> Square<T> {
    pub fn aligned(&self, alignment: valued::Alignment) -> &T {
        match alignment {
            valued::Alignment::LEFT => &self.left,
            valued::Alignment::RIGHT => &self.right,
            valued::Alignment::TOP => &self.top,
            valued::Alignment::BOTTOM => &self.bottom,
        }
    }
}

impl<T> HorizontalEnvelope<T> for Square<T> {
    fn left(&self) -> &T {
        &self.left
    }

    fn right(&self) -> &T {
        &self.right
    }
}

impl<T> VerticalEnvelope<T> for Square<T> {
    fn top(&self) -> &T {
        &self.top
    }

    fn bottom(&self) -> &T {
        &self.bottom
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Quadrant<T> {
    pub top: Horizontal<T>,
    pub bottom: Horizontal<T>,
}

impl<T> Quadrant<T> {
    pub fn aligned(
        &self,
        vertical: valued::VerticalAlignment,
        horizontal: valued::HorizontalAlignment,
    ) -> &T {
        match vertical {
            valued::VerticalAlignment::Top => self.top.aligned(horizontal),
            valued::VerticalAlignment::Bottom => self.bottom.aligned(horizontal),
        }
    }
}

impl<T> From<Horizontal<Vertical<T>>> for Quadrant<T> {
    fn from(horizontal: Horizontal<Vertical<T>>) -> Self {
        horizontal.transpose().into()
    }
}

impl<T> From<Vertical<Horizontal<T>>> for Quadrant<T> {
    fn from(vertical: Vertical<Horizontal<T>>) -> Self {
        let Vertical { top, bottom } = vertical;
        Quadrant { top, bottom }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Axial<T> {
    pub horizontal: T,
    pub vertical: T,
}

impl<T> AxialEnvelope<T> for Axial<T> {
    fn horizontal(&self) -> &T {
        &self.horizontal
    }

    fn vertical(&self) -> &T {
        &self.vertical
    }
}
