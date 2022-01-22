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
