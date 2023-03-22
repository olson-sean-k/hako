#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Axis {
    LeftRight,
    TopBottom,
}

impl Axis {
    pub const fn origin(&self) -> Alignment {
        match *self {
            Axis::LeftRight => Alignment::LEFT,
            Axis::TopBottom => Alignment::TOP,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Alignment {
    Horizontal(HorizontalAlignment),
    Vertical(VerticalAlignment),
}

impl Alignment {
    pub const LEFT: Self = Alignment::Horizontal(HorizontalAlignment::Left);
    pub const RIGHT: Self = Alignment::Horizontal(HorizontalAlignment::Right);
    pub const TOP: Self = Alignment::Vertical(VerticalAlignment::Top);
    pub const BOTTOM: Self = Alignment::Vertical(VerticalAlignment::Bottom);

    #[must_use]
    pub const fn opposite(&self) -> Self {
        match *self {
            Self::LEFT => Self::RIGHT,
            Self::RIGHT => Self::LEFT,
            Self::TOP => Self::BOTTOM,
            Self::BOTTOM => Self::TOP,
        }
    }

    pub const fn axis(&self) -> Axis {
        match *self {
            Self::LEFT | Self::RIGHT => Axis::LeftRight,
            Self::TOP | Self::BOTTOM => Axis::TopBottom,
        }
    }

    pub fn is_left(&self) -> bool {
        matches!(self, Alignment::Horizontal(HorizontalAlignment::Left))
    }

    pub fn is_right(&self) -> bool {
        matches!(self, Alignment::Horizontal(HorizontalAlignment::Right))
    }

    pub fn is_top(&self) -> bool {
        matches!(self, Alignment::Vertical(VerticalAlignment::Top))
    }

    pub fn is_bottom(&self) -> bool {
        matches!(self, Alignment::Vertical(VerticalAlignment::Bottom))
    }
}

impl From<HorizontalAlignment> for Alignment {
    fn from(horizontal: HorizontalAlignment) -> Self {
        Alignment::Horizontal(horizontal)
    }
}

impl From<VerticalAlignment> for Alignment {
    fn from(vertical: VerticalAlignment) -> Self {
        Alignment::Vertical(vertical)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum HorizontalAlignment {
    Left,
    Right,
}

impl HorizontalAlignment {
    pub const AXIS: Axis = Axis::LeftRight;

    #[must_use]
    pub const fn opposite(&self) -> Self {
        match *self {
            HorizontalAlignment::Left => HorizontalAlignment::Right,
            HorizontalAlignment::Right => HorizontalAlignment::Left,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum VerticalAlignment {
    Top,
    Bottom,
}

impl VerticalAlignment {
    pub const AXIS: Axis = Axis::TopBottom;

    #[must_use]
    pub const fn opposite(&self) -> Self {
        match *self {
            VerticalAlignment::Top => VerticalAlignment::Bottom,
            VerticalAlignment::Bottom => VerticalAlignment::Top,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AxialAlignment {
    LeftRight(VerticalAlignment),
    TopBottom(HorizontalAlignment),
}

impl AxialAlignment {
    pub const LEFT_RIGHT_AT_TOP: Self = AxialAlignment::LeftRight(VerticalAlignment::Top);
    pub const LEFT_RIGHT_AT_BOTTOM: Self = AxialAlignment::LeftRight(VerticalAlignment::Bottom);
    pub const TOP_BOTTOM_AT_LEFT: Self = AxialAlignment::TopBottom(HorizontalAlignment::Left);
    pub const TOP_BOTTOM_AT_RIGHT: Self = AxialAlignment::TopBottom(HorizontalAlignment::Right);
}
