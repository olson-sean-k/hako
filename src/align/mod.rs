pub mod typed;
pub mod valued;

pub trait HorizontallyAligned<T>: Sized {
    fn left(&self) -> &T;

    fn right(&self) -> &T;

    fn aligned_at<H>(&self) -> &T
    where
        H: typed::HorizontalAlignment,
    {
        H::aligned(self)
    }
}

pub trait VerticallyAligned<T>: Sized {
    fn top(&self) -> &T;

    fn bottom(&self) -> &T;

    fn aligned_at<V>(&self) -> &T
    where
        V: typed::VerticalAlignment,
    {
        V::aligned(self)
    }
}

pub trait AxiallyAligned<T>: Sized {
    fn horizontal(&self) -> &T;

    fn vertical(&self) -> &T;

    fn aligned_at<A>(&self) -> &T
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

    pub fn with<H, U, F>(&self, mut f: F) -> U
    where
        H: typed::HorizontalAlignment,
        H::Opposite: typed::HorizontalAlignment,
        F: FnMut(&T, &T) -> U,
    {
        f(self.aligned_at::<H>(), self.aligned_at::<H::Opposite>())
    }
}

impl<T> HorizontallyAligned<T> for Horizontal<T> {
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

    pub fn with<V, U, F>(&self, mut f: F) -> U
    where
        V: typed::VerticalAlignment,
        V::Opposite: typed::VerticalAlignment,
        F: FnMut(&T, &T) -> U,
    {
        f(self.aligned_at::<V>(), self.aligned_at::<V::Opposite>())
    }
}

impl<T> VerticallyAligned<T> for Vertical<T> {
    fn top(&self) -> &T {
        &self.top
    }

    fn bottom(&self) -> &T {
        &self.bottom
    }
}

pub type Cornered<T> = Vertical<Horizontal<T>>;

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

impl<T> HorizontallyAligned<T> for Square<T> {
    fn left(&self) -> &T {
        &self.left
    }

    fn right(&self) -> &T {
        &self.right
    }
}

impl<T> VerticallyAligned<T> for Square<T> {
    fn top(&self) -> &T {
        &self.top
    }

    fn bottom(&self) -> &T {
        &self.bottom
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Axial<T> {
    pub horizontal: T,
    pub vertical: T,
}

impl<T> AxiallyAligned<T> for Axial<T> {
    fn horizontal(&self) -> &T {
        &self.horizontal
    }

    fn vertical(&self) -> &T {
        &self.vertical
    }
}
