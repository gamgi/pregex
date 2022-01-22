use crate::nfa::{State, StateParams};
use itertools::Itertools;
use std::collections::HashMap;

pub fn probs(params: &HashMap<usize, StateParams>) -> Vec<f64> {
    params
        .keys()
        .sorted()
        .map(|k| params[k].p)
        .collect::<Vec<f64>>()
}

pub fn find_max<'a, I>(vals: I) -> f64
where
    I: Iterator<Item = f64>,
{
    vals.fold(f64::NEG_INFINITY, |a, b| a.max(b))
}
