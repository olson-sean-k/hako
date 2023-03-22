use crate::align::{AxiallyAligned, HorizontallyAligned, VerticallyAligned};

pub trait HorizontalDecoder {
    fn aligned<T>(data: &impl HorizontallyAligned<T>) -> &T;
}

pub trait VerticalDecoder {
    fn aligned<T>(data: &impl VerticallyAligned<T>) -> &T;
}

pub trait AxialDecoder {
    fn aligned<T>(data: &impl AxiallyAligned<T>) -> &T;
}
