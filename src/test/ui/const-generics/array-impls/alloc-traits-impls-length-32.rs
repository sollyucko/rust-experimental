// check-pass

pub fn yes_vec_partial_eq_array<A, B>() -> impl PartialEq<[B; 32]>
where
    A: PartialEq<B>,
{
    Vec::<A>::new()
}

pub fn yes_vec_partial_eq_ref_array<'a, A, B>() -> impl PartialEq<&'a [B; 32]>
where
    A: PartialEq<B>,
{
    Vec::<A>::new()
}

// We can't create the array since it can't be empty and we don't know how to create As.
pub fn yes_array_partial_eq_vec<A, B>(array: [A; 32]) -> impl PartialEq<Vec<B>>
where
    A: PartialEq<B>,
{
    array
}

pub fn yes_ref_array_partial_eq_vec<'a, A, B>(ref_array: &'a [A; 32]) -> impl PartialEq<Vec<B>> + 'a
where
    A: PartialEq<B>,
{
    ref_array
}

pub fn yes_ref_mut_array_partial_eq_vec<'a, A, B>(
    ref_mut_array: &'a mut [A; 32],
) -> impl PartialEq<Vec<B>> + 'a
where
    A: PartialEq<B>,
{
    ref_mut_array
}

pub fn yes_array_into_vec<T>() -> Vec<T> {
    [].into()
}

use std::collections::VecDeque;

pub fn yes_vecdeque_partial_eq_array<A, B>() -> impl PartialEq<[B; 32]>
where
    A: PartialEq<B>,
{
    VecDeque::<A>::new()
}

pub fn yes_vecdeque_partial_eq_ref_array<'a, A, B>() -> impl PartialEq<&'a [B; 32]>
where
    A: PartialEq<B>,
{
    VecDeque::<A>::new()
}

pub fn yes_vecdeque_partial_eq_ref_mut_array<'a, A, B>() -> impl PartialEq<&'a mut [B; 32]>
where
    A: PartialEq<B>,
{
    VecDeque::<A>::new()
}

fn main() {}
