mod decoder;

use crate::align::typed::decoder::{AxialDecoder, HorizontalDecoder, VerticalDecoder};
use crate::align::{valued, AxiallyAligned, HorizontallyAligned, VerticallyAligned};

pub type OrthogonalOrigin<A> = <<A as Axis>::Orthogonal as Axis>::Origin;

pub trait Axis: AxialDecoder + Sized {
    type Orthogonal: Axis;
    type Origin: Coaxial<Self>;

    const VALUE: valued::Axis;
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

    const VALUE: valued::Axis = valued::Axis::LeftRight;
}

impl AxialDecoder for TopBottom {
    fn aligned<T>(data: &impl AxiallyAligned<T>) -> &T {
        data.vertical()
    }
}

impl Axis for TopBottom {
    type Orthogonal = LeftRight;
    type Origin = Top;

    const VALUE: valued::Axis = valued::Axis::TopBottom;
}

pub trait Alignment {
    type Opposite: Coaxial<Self::Axis>;
    type Axis: Axis;

    const VALUE: valued::Alignment;
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

pub enum Left {}
pub enum Right {}
pub enum Top {}
pub enum Bottom {}

impl Alignment for Left {
    type Opposite = Right;
    type Axis = LeftRight;

    const VALUE: valued::Alignment = valued::Alignment::LEFT;
}

impl HorizontalDecoder for Left {
    fn aligned<T>(data: &impl HorizontallyAligned<T>) -> &T {
        data.left()
    }
}

impl Alignment for Right {
    type Opposite = Left;
    type Axis = LeftRight;

    const VALUE: valued::Alignment = valued::Alignment::RIGHT;
}

impl HorizontalDecoder for Right {
    fn aligned<T>(data: &impl HorizontallyAligned<T>) -> &T {
        data.right()
    }
}

impl Alignment for Top {
    type Opposite = Bottom;
    type Axis = TopBottom;

    const VALUE: valued::Alignment = valued::Alignment::TOP;
}

impl VerticalDecoder for Top {
    fn aligned<T>(data: &impl VerticallyAligned<T>) -> &T {
        data.top()
    }
}

impl Alignment for Bottom {
    type Opposite = Top;
    type Axis = TopBottom;

    const VALUE: valued::Alignment = valued::Alignment::BOTTOM;
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
