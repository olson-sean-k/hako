use std::ops::Index;

pub trait SliceIterator: DoubleEndedIterator + ExactSizeIterator {}

impl<I> SliceIterator for I where I: DoubleEndedIterator + ExactSizeIterator {}

pub trait SliceExt<T> {
    fn project<'a, U, F>(&'a self, f: F) -> ProjectedSlice<'a, T, F>
    where
        U: 'a,
        F: Fn(&'a T) -> &'a U;
}

impl<T> SliceExt<T> for [T] {
    fn project<'a, U, F>(&'a self, f: F) -> ProjectedSlice<'a, T, F>
    where
        U: 'a,
        F: Fn(&'a T) -> &'a U,
    {
        ProjectedSlice { slice: self, f }
    }
}

pub trait SliceProjection: Index<usize, Output = Self::Item> {
    type Item;

    fn get(&self, index: usize) -> Option<&Self::Item>;

    fn iter(&self) -> impl '_ + Clone + SliceIterator<Item = &'_ Self::Item>;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T> SliceProjection for [T] {
    type Item = T;

    fn get(&self, index: usize) -> Option<&Self::Item> {
        self.get(index)
    }

    fn iter(&self) -> impl '_ + Clone + SliceIterator<Item = &'_ Self::Item> {
        self.iter()
    }

    fn len(&self) -> usize {
        self.len()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ProjectedSlice<'a, T, F> {
    // TODO: Generalize this such that `SliceExt` is replaced by `SliceProjection::project` and
    //       re-projection is possible.
    slice: &'a [T],
    f: F,
}

// TODO: Support `SliceIndex` and re-projecting outputs when indexed over a range.
impl<'a, T, U, F> Index<usize> for ProjectedSlice<'a, T, F>
where
    U: 'a,
    F: Fn(&'a T) -> &'a U,
{
    type Output = U;

    fn index(&self, index: usize) -> &Self::Output {
        (self.f)(&self.slice[index])
    }
}

impl<'a, T, U, F> SliceProjection for ProjectedSlice<'a, T, F>
where
    U: 'a,
    F: Fn(&'a T) -> &'a U,
{
    type Item = U;

    fn get(&self, index: usize) -> Option<&Self::Item> {
        self.slice.get(index).map(|item| (self.f)(item))
    }

    fn iter(&self) -> impl '_ + Clone + SliceIterator<Item = &'_ Self::Item> {
        self.slice.iter().map(|item| (self.f)(item))
    }

    fn len(&self) -> usize {
        self.slice.len()
    }
}
