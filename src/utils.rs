use itertools::Itertools;
use crate::distribution::StateParams;
use std::collections::HashMap;

pub fn probs(params: &HashMap<usize, StateParams>) -> Vec<f64> {
    params 
    .keys()
    .sorted()
    .map(|k| params[k].1)
    .collect::<Vec<f64>>()
}