use itertools::Position;

pub type OrthogonalOrigin<A> = <<A as Axis>::Orthogonal as Axis>::Origin;

pub trait Axis: Sized {
    type Orthogonal: Axis;
    type Origin: Coaxial<Self>;

    const VALUE: AxisValue;
}

pub enum LeftRight {}
pub enum TopBottom {}

impl Axis for LeftRight {
    type Orthogonal = TopBottom;
    type Origin = Left;

    const VALUE: AxisValue = AxisValue::LeftRight;
}

impl Axis for TopBottom {
    type Orthogonal = LeftRight;
    type Origin = Top;

    const VALUE: AxisValue = AxisValue::TopBottom;
}

pub trait Alignment {
    type Opposite: Coaxial<Self::Axis>;
    type Axis: Axis;

    const VALUE: AlignmentValue;
}

pub trait HorizontalAlignment:
    Coaxial<LeftRight> + ContraAxial<TopBottom> + HorizontalDecoder
{
}

impl<L> HorizontalAlignment for L where
    L: Coaxial<LeftRight> + ContraAxial<TopBottom> + HorizontalDecoder
{
}

pub trait VerticalAlignment: Coaxial<TopBottom> + ContraAxial<LeftRight> + VerticalDecoder {}

impl<L> VerticalAlignment for L where
    L: Coaxial<TopBottom> + ContraAxial<LeftRight> + VerticalDecoder
{
}

pub trait HorizontalDecoder {
    fn aligned<T>(data: &impl HorizontallyAligned<T>) -> &T;
}

pub trait VerticalDecoder {
    fn aligned<T>(data: &impl VerticallyAligned<T>) -> &T;
}

pub enum Left {}
pub enum Right {}
pub enum Top {}
pub enum Bottom {}

impl Alignment for Left {
    type Opposite = Right;
    type Axis = LeftRight;

    const VALUE: AlignmentValue = AlignmentValue::LEFT;
}

impl HorizontalDecoder for Left {
    fn aligned<T>(data: &impl HorizontallyAligned<T>) -> &T {
        data.left()
    }
}

impl Alignment for Right {
    type Opposite = Left;
    type Axis = LeftRight;

    const VALUE: AlignmentValue = AlignmentValue::RIGHT;
}

impl HorizontalDecoder for Right {
    fn aligned<T>(data: &impl HorizontallyAligned<T>) -> &T {
        data.right()
    }
}

impl Alignment for Top {
    type Opposite = Bottom;
    type Axis = TopBottom;

    const VALUE: AlignmentValue = AlignmentValue::TOP;
}

impl VerticalDecoder for Top {
    fn aligned<T>(data: &impl VerticallyAligned<T>) -> &T {
        data.top()
    }
}

impl Alignment for Bottom {
    type Opposite = Top;
    type Axis = TopBottom;

    const VALUE: AlignmentValue = AlignmentValue::BOTTOM;
}

impl VerticalDecoder for Bottom {
    fn aligned<T>(data: &impl VerticallyAligned<T>) -> &T {
        data.bottom()
    }
}

pub trait Coaxial<A>: Alignment<Axis = A>
where
    A: Axis,
{
}

impl<A, L> Coaxial<A> for L
where
    A: Axis,
    L: Alignment<Axis = A>,
{
}

pub trait ContraAxial<A>: Alignment<Axis = <A as Axis>::Orthogonal>
where
    A: Axis,
{
}

impl<A, L> ContraAxial<A> for L
where
    A: Axis,
    L: Alignment<Axis = <A as Axis>::Orthogonal>,
{
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AxisValue {
    LeftRight,
    TopBottom,
}

impl AxisValue {
    pub const fn origin(&self) -> AlignmentValue {
        match *self {
            AxisValue::LeftRight => AlignmentValue::LEFT,
            AxisValue::TopBottom => AlignmentValue::TOP,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AlignmentValue {
    Horizontal(HorizontalAlignmentValue),
    Vertical(VerticalAlignmentValue),
}

impl AlignmentValue {
    pub const LEFT: Self = AlignmentValue::Horizontal(HorizontalAlignmentValue::Left);
    pub const RIGHT: Self = AlignmentValue::Horizontal(HorizontalAlignmentValue::Right);
    pub const TOP: Self = AlignmentValue::Vertical(VerticalAlignmentValue::Top);
    pub const BOTTOM: Self = AlignmentValue::Vertical(VerticalAlignmentValue::Bottom);

    #[must_use]
    pub const fn opposite(&self) -> Self {
        match *self {
            Self::LEFT => Self::RIGHT,
            Self::RIGHT => Self::LEFT,
            Self::TOP => Self::BOTTOM,
            Self::BOTTOM => Self::TOP,
        }
    }

    pub const fn axis(&self) -> AxisValue {
        match *self {
            Self::LEFT | Self::RIGHT => AxisValue::LeftRight,
            Self::TOP | Self::BOTTOM => AxisValue::TopBottom,
        }
    }

    pub fn is_left(&self) -> bool {
        matches!(
            self,
            AlignmentValue::Horizontal(HorizontalAlignmentValue::Left)
        )
    }

    pub fn is_right(&self) -> bool {
        matches!(
            self,
            AlignmentValue::Horizontal(HorizontalAlignmentValue::Right)
        )
    }

    pub fn is_top(&self) -> bool {
        matches!(self, AlignmentValue::Vertical(VerticalAlignmentValue::Top))
    }

    pub fn is_bottom(&self) -> bool {
        matches!(
            self,
            AlignmentValue::Vertical(VerticalAlignmentValue::Bottom)
        )
    }
}

impl From<HorizontalAlignmentValue> for AlignmentValue {
    fn from(horizontal: HorizontalAlignmentValue) -> Self {
        AlignmentValue::Horizontal(horizontal)
    }
}

impl From<VerticalAlignmentValue> for AlignmentValue {
    fn from(vertical: VerticalAlignmentValue) -> Self {
        AlignmentValue::Vertical(vertical)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum HorizontalAlignmentValue {
    Left,
    Right,
}

impl HorizontalAlignmentValue {
    pub const AXIS: AxisValue = AxisValue::LeftRight;

    #[must_use]
    pub const fn opposite(&self) -> Self {
        match *self {
            HorizontalAlignmentValue::Left => HorizontalAlignmentValue::Right,
            HorizontalAlignmentValue::Right => HorizontalAlignmentValue::Left,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum VerticalAlignmentValue {
    Top,
    Bottom,
}

impl VerticalAlignmentValue {
    pub const AXIS: AxisValue = AxisValue::TopBottom;

    #[must_use]
    pub const fn opposite(&self) -> Self {
        match *self {
            VerticalAlignmentValue::Top => VerticalAlignmentValue::Bottom,
            VerticalAlignmentValue::Bottom => VerticalAlignmentValue::Top,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AxialAlignmentValue {
    LeftRight(VerticalAlignmentValue),
    TopBottom(HorizontalAlignmentValue),
}

impl AxialAlignmentValue {
    pub const LEFT_RIGHT_AT_TOP: Self = AxialAlignmentValue::LeftRight(VerticalAlignmentValue::Top);
    pub const LEFT_RIGHT_AT_BOTTOM: Self =
        AxialAlignmentValue::LeftRight(VerticalAlignmentValue::Bottom);
    pub const TOP_BOTTOM_AT_LEFT: Self =
        AxialAlignmentValue::TopBottom(HorizontalAlignmentValue::Left);
    pub const TOP_BOTTOM_AT_RIGHT: Self =
        AxialAlignmentValue::TopBottom(HorizontalAlignmentValue::Right);
}

pub trait HorizontallyAligned<T>: Sized {
    fn left(&self) -> &T;

    fn right(&self) -> &T;

    fn aligned_at<H>(&self) -> &T
    where
        H: HorizontalDecoder,
    {
        H::aligned(self)
    }
}

pub trait VerticallyAligned<T>: Sized {
    fn top(&self) -> &T;

    fn bottom(&self) -> &T;

    fn aligned_at<V>(&self) -> &T
    where
        V: VerticalDecoder,
    {
        V::aligned(self)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Horizontal<T> {
    pub left: T,
    pub right: T,
}

impl<T> Horizontal<T> {
    pub fn aligned(&self, alignment: HorizontalAlignmentValue) -> &T {
        match alignment {
            HorizontalAlignmentValue::Left => &self.left,
            HorizontalAlignmentValue::Right => &self.left,
        }
    }

    pub fn with<H, U, F>(&self, mut f: F) -> U
    where
        H: HorizontalAlignment,
        H::Opposite: HorizontalAlignment,
        F: FnMut(&T, &T) -> U,
    {
        f(self.aligned_at::<H>(), self.aligned_at::<H::Opposite>())
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
    pub fn aligned(&self, alignment: VerticalAlignmentValue) -> &T {
        match alignment {
            VerticalAlignmentValue::Top => &self.top,
            VerticalAlignmentValue::Bottom => &self.bottom,
        }
    }

    pub fn with<V, U, F>(&self, mut f: F) -> U
    where
        V: VerticalAlignment,
        V::Opposite: VerticalAlignment,
        F: FnMut(&T, &T) -> U,
    {
        f(self.aligned_at::<V>(), self.aligned_at::<V::Opposite>())
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
    pub fn aligned(&self, alignment: AlignmentValue) -> &T {
        match alignment {
            AlignmentValue::LEFT => &self.left,
            AlignmentValue::RIGHT => &self.right,
            AlignmentValue::TOP => &self.top,
            AlignmentValue::BOTTOM => &self.bottom,
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
