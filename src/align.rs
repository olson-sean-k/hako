use itertools::Position;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AxisValue {
    LeftRight,
    TopBottom,
}

pub trait Axis {
    type Orthogonal: Axis;
}

pub enum LeftRight {}
pub enum TopBottom {}

pub type UpDown = TopBottom;

impl Axis for LeftRight {
    type Orthogonal = TopBottom;
}

impl Axis for TopBottom {
    type Orthogonal = LeftRight;
}

pub trait Alignment {
    type Opposite: Alignment;
    type Axis: Axis;
}

pub trait HorizontalAlignment: Alignment {}

pub trait VerticalAlignment: Alignment {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AlignmentValue {
    Horizontal(HorizontalAlignmentValue),
    Vertical(VerticalAlignmentValue),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum HorizontalAlignmentValue {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum VerticalAlignmentValue {
    Top,
    Bottom,
}

pub enum Left {}
pub enum Right {}
pub enum Top {}
pub enum Bottom {}

pub type Up = Top;
pub type Down = Bottom;

impl Alignment for Left {
    type Opposite = Right;
    type Axis = LeftRight;
}

impl HorizontalAlignment for Left {}

impl Alignment for Right {
    type Opposite = Left;
    type Axis = LeftRight;
}

impl HorizontalAlignment for Right {}

impl Alignment for Top {
    type Opposite = Bottom;
    type Axis = TopBottom;
}

impl VerticalAlignment for Top {}

impl Alignment for Bottom {
    type Opposite = Top;
    type Axis = TopBottom;
}

impl VerticalAlignment for Bottom {}

pub trait AxisAligned<A>
where
    A: Alignment,
{
    type Output;

    fn axis_aligned(&self) -> &Self::Output;
}

pub trait AxisAlignedOf {
    fn axis_aligned_of<A>(&self) -> &Self::Output
    where
        Self: AxisAligned<A>,
        A: Alignment,
    {
        AxisAligned::<A>::axis_aligned(self)
    }
}

impl<T> AxisAlignedOf for T {}

pub trait QuadrantAligned<V, H>
where
    V: VerticalAlignment,
    H: HorizontalAlignment,
{
    type Output;

    fn quadrant_aligned(&self) -> &Self::Output;
}

pub trait QuadrantAlignedOf {
    fn quadrant_aligned_of<V, H>(&self) -> &Self::Output
    where
        Self: QuadrantAligned<V, H>,
        V: VerticalAlignment,
        H: HorizontalAlignment,
    {
        QuadrantAligned::<V, H>::quadrant_aligned(self)
    }
}

impl<T> QuadrantAlignedOf for T {}

pub trait AxisOrdered<A>: Sized
where
    A: Alignment,
{
    type Output: IntoIterator<Item = Self::Item>;
    type Item;

    fn axis_ordered(self) -> Self::Output;
}

pub trait AxisOrderedOf: Sized {
    fn axis_ordered_of<A>(self) -> Self::Output
    where
        Self: AxisOrdered<A>,
        A: Alignment,
    {
        AxisOrdered::<A>::axis_ordered(self)
    }
}

impl<T> AxisOrderedOf for T {}

#[derive(Clone, Copy, Debug)]
pub struct Horizontal<T> {
    pub left: T,
    pub right: T,
}

impl<T> Horizontal<T> {
    pub fn with<A, U, F>(&self, mut f: F) -> U
    where
        Self: AxisAligned<A, Output = T>,
        Self: AxisAligned<A::Opposite, Output = T>,
        A: HorizontalAlignment,
        F: FnMut(&T, &T) -> U,
    {
        f(
            self.axis_aligned_of::<A>(),
            self.axis_aligned_of::<A::Opposite>(),
        )
    }
}

impl<T> AxisAligned<Left> for Horizontal<T> {
    type Output = T;

    fn axis_aligned(&self) -> &Self::Output {
        &self.left
    }
}

impl<T> AxisAligned<Right> for Horizontal<T> {
    type Output = T;

    fn axis_aligned(&self) -> &Self::Output {
        &self.right
    }
}

impl<T> AxisOrdered<Left> for Horizontal<T> {
    type Output = [T; 2];
    type Item = T;

    fn axis_ordered(self) -> Self::Output {
        [self.left, self.right]
    }
}

impl<T> AxisOrdered<Right> for Horizontal<T> {
    type Output = [T; 2];
    type Item = T;

    fn axis_ordered(self) -> Self::Output {
        [self.right, self.left]
    }
}

// TODO: This is odd, but allows `First` and `Last` to swap. Perhaps use a
//       bespoke extension trait for this conversion instead.
impl<T> From<Position<T>> for Horizontal<Position<T>>
where
    T: Copy,
{
    fn from(position: Position<T>) -> Horizontal<Position<T>> {
        match position {
            Position::Only(_) => Horizontal {
                left: position,
                right: position,
            },
            Position::First(inner) => Horizontal {
                left: position,
                right: Position::Last(inner),
            },
            Position::Middle(_) => Horizontal {
                left: position,
                right: position,
            },
            Position::Last(inner) => Horizontal {
                left: position,
                right: Position::First(inner),
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Vertical<T> {
    pub top: T,
    pub bottom: T,
}

impl<T> Vertical<T> {
    pub fn with<A, U, F>(&self, mut f: F) -> U
    where
        Self: AxisAligned<A, Output = T>,
        Self: AxisAligned<A::Opposite, Output = T>,
        A: VerticalAlignment,
        F: FnMut(&T, &T) -> U,
    {
        f(
            self.axis_aligned_of::<A>(),
            self.axis_aligned_of::<A::Opposite>(),
        )
    }
}

impl<T> AxisAligned<Top> for Vertical<T> {
    type Output = T;

    fn axis_aligned(&self) -> &Self::Output {
        &self.top
    }
}

impl<T> AxisAligned<Bottom> for Vertical<T> {
    type Output = T;

    fn axis_aligned(&self) -> &Self::Output {
        &self.bottom
    }
}

impl<T> AxisOrdered<Top> for Vertical<T> {
    type Output = [T; 2];
    type Item = T;

    fn axis_ordered(self) -> Self::Output {
        [self.top, self.bottom]
    }
}

impl<T> AxisOrdered<Bottom> for Vertical<T> {
    type Output = [T; 2];
    type Item = T;

    fn axis_ordered(self) -> Self::Output {
        [self.bottom, self.top]
    }
}

// TODO: This is odd, but allows `First` and `Last` to swap. Perhaps use a
//       bespoke extension trait for this conversion instead.
impl<T> From<Position<T>> for Vertical<Position<T>>
where
    T: Copy,
{
    fn from(position: Position<T>) -> Vertical<Position<T>> {
        match position {
            Position::Only(_) => Vertical {
                top: position,
                bottom: position,
            },
            Position::First(inner) => Vertical {
                top: position,
                bottom: Position::Last(inner),
            },
            Position::Middle(_) => Vertical {
                top: position,
                bottom: position,
            },
            Position::Last(inner) => Vertical {
                top: position,
                bottom: Position::First(inner),
            },
        }
    }
}

pub type Cornered<T> = Vertical<Horizontal<T>>;

impl<T> QuadrantAligned<Top, Left> for Cornered<T> {
    type Output = T;

    fn quadrant_aligned(&self) -> &Self::Output {
        &self.top.left
    }
}

impl<T> QuadrantAligned<Top, Right> for Cornered<T> {
    type Output = T;

    fn quadrant_aligned(&self) -> &Self::Output {
        &self.top.right
    }
}

impl<T> QuadrantAligned<Bottom, Left> for Cornered<T> {
    type Output = T;

    fn quadrant_aligned(&self) -> &Self::Output {
        &self.bottom.left
    }
}

impl<T> QuadrantAligned<Bottom, Right> for Cornered<T> {
    type Output = T;

    fn quadrant_aligned(&self) -> &Self::Output {
        &self.bottom.right
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
    pub fn with<A, U, F>(&self, mut f: F) -> U
    where
        Self: AxisAligned<A, Output = T>,
        Self: AxisAligned<A::Opposite, Output = T>,
        A: Alignment,
        F: FnMut(&T, &T) -> U,
    {
        f(
            self.axis_aligned_of::<A>(),
            self.axis_aligned_of::<A::Opposite>(),
        )
    }
}

impl<T> AxisAligned<Left> for Square<T> {
    type Output = T;

    fn axis_aligned(&self) -> &Self::Output {
        &self.left
    }
}

impl<T> AxisAligned<Right> for Square<T> {
    type Output = T;

    fn axis_aligned(&self) -> &Self::Output {
        &self.right
    }
}

impl<T> AxisAligned<Top> for Square<T> {
    type Output = T;

    fn axis_aligned(&self) -> &Self::Output {
        &self.top
    }
}

impl<T> AxisAligned<Bottom> for Square<T> {
    type Output = T;

    fn axis_aligned(&self) -> &Self::Output {
        &self.bottom
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Axial<T> {
    pub horizontal: T,
    pub vertical: T,
}
