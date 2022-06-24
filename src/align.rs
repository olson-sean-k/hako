pub type OrthogonalOrigin<A> = <<A as Axis>::Orthogonal as Axis>::Origin;

pub trait Axis: AxialDecoder + Sized {
    type Orthogonal: Axis;
    type Origin: Coaxial<Self>;

    const VALUE: AxisValue;
}

pub enum LeftRight {}
pub enum TopBottom {}

impl AxialDecoder for LeftRight {
    fn aligned<T>(data: &impl AxiallyAligned<T>) -> &T {
        data.horizontal()
    }
}

impl Axis for LeftRight {
    type Orthogonal = TopBottom;
    type Origin = Left;

    const VALUE: AxisValue = AxisValue::LeftRight;
}

impl AxialDecoder for TopBottom {
    fn aligned<T>(data: &impl AxiallyAligned<T>) -> &T {
        data.vertical()
    }
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

// TODO: Do not export this trait. It is an implementation detail with which
//       client code need not directly interact nor understand.
pub trait HorizontalDecoder {
    fn aligned<T>(data: &impl HorizontallyAligned<T>) -> &T;
}

// TODO: Do not export this trait. It is an implementation detail with which
//       client code need not directly interact nor understand.
pub trait VerticalDecoder {
    fn aligned<T>(data: &impl VerticallyAligned<T>) -> &T;
}

// TODO: Do not export this trait. It is an implementation detail with which
//       client code need not directly interact nor understand.
pub trait AxialDecoder {
    fn aligned<T>(data: &impl AxiallyAligned<T>) -> &T;
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

pub trait Oriented {
    type Origin: Alignment;
}

pub trait Rotate<L>: Oriented
where
    L: Alignment,
{
    type Output;

    fn rotate(self) -> Self::Output;
}

impl<T> Rotate<<T as Oriented>::Origin> for T
where
    T: Oriented,
{
    type Output = Self;

    fn rotate(self) -> Self::Output {
        self
    }
}

pub trait Invert: Sized {
    fn invert(self) -> Self;
}

impl<T> Invert for T
where
    T: Rotate<<<T as Oriented>::Origin as Alignment>::Opposite, Output = Self>,
{
    fn invert(self) -> Self {
        self.rotate()
    }
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
        H: HorizontalAlignment,
    {
        H::aligned(self)
    }
}

pub trait VerticallyAligned<T>: Sized {
    fn top(&self) -> &T;

    fn bottom(&self) -> &T;

    fn aligned_at<V>(&self) -> &T
    where
        V: VerticalAlignment,
    {
        V::aligned(self)
    }
}

pub trait AxiallyAligned<T>: Sized {
    fn horizontal(&self) -> &T;

    fn vertical(&self) -> &T;

    fn aligned_at<A>(&self) -> &T
    where
        A: Axis,
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
    pub fn aligned(&self, alignment: HorizontalAlignmentValue) -> &T {
        match alignment {
            HorizontalAlignmentValue::Left => &self.left,
            HorizontalAlignmentValue::Right => &self.right,
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

impl<T> HorizontallyAligned<T> for Horizontal<T> {
    fn left(&self) -> &T {
        &self.left
    }

    fn right(&self) -> &T {
        &self.right
    }
}

impl<T> Oriented for Horizontal<T> {
    type Origin = <LeftRight as Axis>::Origin;
}

impl<T> Rotate<Right> for Horizontal<T> {
    type Output = Self;

    fn rotate(self) -> Self::Output {
        let Horizontal { left, right } = self;
        Horizontal {
            left: right,
            right: left,
        }
    }
}

impl<T> Rotate<Top> for Horizontal<T> {
    type Output = Vertical<T>;

    fn rotate(self) -> Self::Output {
        let Horizontal { left, right } = self;
        Vertical {
            top: left,
            bottom: right,
        }
    }
}

impl<T> Rotate<Bottom> for Horizontal<T> {
    type Output = Vertical<T>;

    fn rotate(self) -> Self::Output {
        let Horizontal { left, right } = self;
        Vertical {
            top: right,
            bottom: left,
        }
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

impl<T> Oriented for Vertical<T> {
    type Origin = <TopBottom as Axis>::Origin;
}

impl<T> Rotate<Left> for Vertical<T> {
    type Output = Horizontal<T>;

    fn rotate(self) -> Self::Output {
        let Vertical { top, bottom } = self;
        Horizontal {
            left: top,
            right: bottom,
        }
    }
}

impl<T> Rotate<Right> for Vertical<T> {
    type Output = Horizontal<T>;

    fn rotate(self) -> Self::Output {
        let Vertical { top, bottom } = self;
        Horizontal {
            left: bottom,
            right: top,
        }
    }
}

impl<T> Rotate<Bottom> for Vertical<T> {
    type Output = Self;

    fn rotate(self) -> Self::Output {
        let Vertical { top, bottom } = self;
        Vertical {
            top: bottom,
            bottom: top,
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

#[derive(Clone, Copy, Debug)]
pub struct Quadrant<T> {
    pub top: Horizontal<T>,
    pub bottom: Horizontal<T>,
}

impl<T> Quadrant<T> {
    pub fn aligned(
        &self,
        vertical: VerticalAlignmentValue,
        horizontal: HorizontalAlignmentValue,
    ) -> &T {
        use HorizontalAlignmentValue::{Left, Right};
        use VerticalAlignmentValue::{Bottom, Top};

        match (vertical, horizontal) {
            (Bottom, Left) => &self.bottom.left,
            (Bottom, Right) => &self.bottom.right,
            (Top, Left) => &self.top.left,
            (Top, Right) => &self.top.right,
        }
    }
}

impl<T> From<Vertical<Horizontal<T>>> for Quadrant<T> {
    fn from(vertical: Vertical<Horizontal<T>>) -> Self {
        let Vertical { top, bottom } = vertical;
        Quadrant { top, bottom }
    }
}

impl<T> Oriented for Quadrant<T> {
    type Origin = Top;
}

impl<T> Rotate<Left> for Quadrant<T> {
    type Output = Self;

    fn rotate(self) -> Self::Output {
        let Quadrant { top, bottom } = self;
        let horizontal = Horizontal {
            left: Rotate::<Bottom>::rotate(top),
            right: Rotate::<Bottom>::rotate(bottom),
        };
        horizontal.transpose().into()
    }
}

impl<T> Rotate<Right> for Quadrant<T> {
    type Output = Self;

    fn rotate(self) -> Self::Output {
        let Quadrant { top, bottom } = self;
        let horizontal = Horizontal {
            left: Rotate::<Top>::rotate(top),
            right: Rotate::<Top>::rotate(bottom),
        };
        horizontal.transpose().into()
    }
}

impl<T> Rotate<Bottom> for Quadrant<T> {
    type Output = Self;

    fn rotate(self) -> Self::Output {
        let Quadrant { top, bottom } = self;
        Quadrant {
            top: bottom.invert(),
            bottom: top.invert(),
        }
    }
}

impl<T> VerticallyAligned<Horizontal<T>> for Quadrant<T> {
    fn top(&self) -> &Horizontal<T> {
        &self.top
    }

    fn bottom(&self) -> &Horizontal<T> {
        &self.bottom
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Perimeter<T> {
    pub left: T,
    pub right: T,
    pub top: T,
    pub bottom: T,
}

impl<T> Perimeter<T> {
    pub fn aligned(&self, alignment: AlignmentValue) -> &T {
        match alignment {
            AlignmentValue::LEFT => &self.left,
            AlignmentValue::RIGHT => &self.right,
            AlignmentValue::TOP => &self.top,
            AlignmentValue::BOTTOM => &self.bottom,
        }
    }
}

impl<T> HorizontallyAligned<T> for Perimeter<T> {
    fn left(&self) -> &T {
        &self.left
    }

    fn right(&self) -> &T {
        &self.right
    }
}

impl<T> Oriented for Perimeter<T> {
    type Origin = Top;
}

impl<T> Rotate<Left> for Perimeter<T> {
    type Output = Self;

    fn rotate(self) -> Self::Output {
        let Perimeter {
            left,
            right,
            top,
            bottom,
        } = self;
        Perimeter {
            left: top,
            right: bottom,
            top: right,
            bottom: left,
        }
    }
}

impl<T> Rotate<Right> for Perimeter<T> {
    type Output = Self;

    fn rotate(self) -> Self::Output {
        let Perimeter {
            left,
            right,
            top,
            bottom,
        } = self;
        Perimeter {
            left: bottom,
            right: top,
            top: left,
            bottom: right,
        }
    }
}

impl<T> Rotate<Bottom> for Perimeter<T> {
    type Output = Self;

    fn rotate(self) -> Self::Output {
        let Perimeter {
            left,
            right,
            top,
            bottom,
        } = self;
        Perimeter {
            left: right,
            right: left,
            top: bottom,
            bottom: top,
        }
    }
}

impl<T> VerticallyAligned<T> for Perimeter<T> {
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

impl<T> Invert for Axial<T> {
    fn invert(self) -> Self {
        let Axial {
            horizontal,
            vertical,
        } = self;
        Axial {
            horizontal: vertical,
            vertical: horizontal,
        }
    }
}
