use crate::align::{AxialEnvelope, HorizontalEnvelope, VerticalEnvelope};

pub trait HorizontalDecoder {
    fn aligned<T>(data: &impl HorizontalEnvelope<T>) -> &T;
}

pub trait VerticalDecoder {
    fn aligned<T>(data: &impl VerticalEnvelope<T>) -> &T;
}

pub trait AxialDecoder {
    fn aligned<T>(data: &impl AxialEnvelope<T>) -> &T;
}
