pub fn no_vec_partial_eq_array<A, B>() -> impl PartialEq<[B; 33]>
//~^ ERROR arrays only have std trait implementations for lengths 0..=32
where
    A: PartialEq<B>,
{
    Vec::<A>::new()
}

pub fn no_vec_partial_eq_ref_array<'a, A, B>() -> impl PartialEq<&'a [B; 33]>
//~^ ERROR arrays only have std trait implementations for lengths 0..=32
where
    A: PartialEq<B>,
{
    Vec::<A>::new()
}

pub fn yes_array_partial_eq_vec<A, B>(array: [A; 33]) -> impl PartialEq<Vec<B>>
//~^ ERROR arrays only have std trait implementations for lengths 0..=32
where
    A: PartialEq<B>,
{
    array
}

pub fn yes_ref_array_partial_eq_vec<'a, A, B>(ref_array: &'a [A; 33]) -> impl PartialEq<Vec<B>> + 'a
//~^ ERROR arrays only have std trait implementations for lengths 0..=32
where
    A: PartialEq<B>,
{
    ref_array
}

pub fn yes_ref_mut_array_partial_eq_vec<'a, A, B>(
    ref_mut_array: &'a mut [A; 33],
) -> impl PartialEq<Vec<B>> + 'a
//~^ ERROR arrays only have std trait implementations for lengths 0..=32
where
    A: PartialEq<B>,
{
    ref_mut_array
}

use std::collections::VecDeque;

pub fn no_vecdeque_partial_eq_array<A, B>() -> impl PartialEq<[B; 33]>
//~^ ERROR arrays only have std trait implementations for lengths 0..=32
where
    A: PartialEq<B>,
{
    VecDeque::<A>::new()
}

pub fn no_vecdeque_partial_eq_ref_array<'a, A, B>() -> impl PartialEq<&'a [B; 33]>
//~^ ERROR arrays only have std trait implementations for lengths 0..=32
where
    A: PartialEq<B>,
{
    VecDeque::<A>::new()
}

pub fn no_vecdeque_partial_eq_ref_mut_array<'a, A, B>() -> impl PartialEq<&'a mut [B; 33]>
//~^ ERROR arrays only have std trait implementations for lengths 0..=32
where
    A: PartialEq<B>,
{
    VecDeque::<A>::new()
}

fn main() {}
