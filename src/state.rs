#![allow(dead_code, unused_variables)]
use crate::ast::{AstNode, Kind};
use crate::nfa::State;
use itertools::Itertools;
use std::collections::{HashMap, HashSet};

#[derive(Debug, PartialEq, PartialOrd)]
pub enum Dist {
    Constant(f32),
    ExactlyTimes(u32),
}

type ActiveState = (Dist, f32, u32); // (c, p, visits)

pub struct NfaState<'a> {
    nfa: &'a Vec<State>,
    visited: HashSet<usize>,
    pub current_states: HashSet<usize>,
    state_params: HashMap<usize, ActiveState>,
}

fn find_max<'a, I>(vals: I) -> f32
where
    I: Iterator<Item = f32>,
{
    vals.fold(f32::NEG_INFINITY, |a, b| a.max(b))
}

impl NfaState<'_> {
    pub fn new(nfa: &Vec<State>) -> NfaState {
        return NfaState {
            nfa: nfa,
            current_states: HashSet::new(),
            visited: HashSet::new(),
            state_params: HashMap::new(),
        };
    }

    pub fn update_states_counts(&mut self, states: &HashMap<usize, f32>) {
        // TODO how is count dependent on p?
        states
            .into_iter()
            .for_each(|(i, p)| self.update_state(*i, 0.0, true));
    }

    pub fn add_states(&mut self, states: &HashMap<usize, f32>, force: bool) -> f32 {
        find_max(
            states
                .into_iter()
                .map(|(i, p)| self.add_state(Some(*i), force, *p)),
        )
    }

    fn get_count(&self, idx: usize) -> u32 {
        if let Some(params) = self.state_params.get(&idx) {
            return params.2;
        }
        0
    }

    fn get_state(&self, i: usize) -> (f32, &State) {
        let p = match self.state_params.get(&i) {
            Some(params) => params.1,
            None => 0.0,
        };
        let state = &self.nfa[i];
        return (p, state);
    }

    fn evaluate_p(&self, i: usize, p: f32) -> (f32, f32) {
        if let Some((dist, _, n)) = self.state_params.get(&i) {
            return match dist {
                Dist::Constant(c) => (p * c, p * c),
                Dist::ExactlyTimes(match_n) => {
                    if n == match_n {
                        (0.0, p)
                    } else if n < match_n {
                        (p, 0.0)
                    } else {
                        (0.0, 0.0)
                    }
                }
            };
        }
        (p, p)
    }

    pub fn add_state(&mut self, idx: Option<usize>, force: bool, p: f32) -> f32 {
        let i = if let Some(i) = idx { i } else { return 0.0 };

        if let Some(state) = self.nfa.get(i) {
            let is_previously_visited = !self.visited.insert(i);
            if is_previously_visited && !force {
                // TODO still update p
                debug!("    skip {}", state.kind);
                return 0.0;
            }
            debug!("    add {} p={}", state.kind, p);

            match state.kind {
                Kind::Terminal => {
                    self.update_state(i, p, false);
                    return p;
                }
                Kind::Quantifier(_) | Kind::Start | Kind::Split | Kind::ExactQuantifier(_) => {
                    let (p0, p1) = self.evaluate_p(i, p);
                    return f32::max(
                        self.add_state(state.outs.0, force, p0),
                        self.add_state(state.outs.1, force, p1),
                    );
                }
                _ => {
                    self.update_state(i, p, false);
                }
            }
        }
        0.0
    }

    pub fn init_state(&mut self, idx: Option<usize>, force: bool) {
        self.add_state(idx, force, 1.0);
        self.visited.drain();
    }

    pub fn step(&mut self, token: char) -> f32 {
        debug!("step {}", token);
        let mut new_states: HashMap<usize, f32> = HashMap::new();
        let mut result = 0.0;

        let mut add_new_state = |idx, p| match idx {
            Some(i) => {
                let e = new_states.entry(i).or_insert(0.0);
                *e = f32::max(*e, p);
            }
            None => {}
        };

        for i in self.current_states.iter() {
            let (p, state) = self.get_state(*i);
            match state.kind {
                Kind::Terminal => {
                    debug!("  terminal");
                    result = f32::max(p, result);
                }
                Kind::Literal(match_token) => {
                    if match_token == token {
                        debug!("  match {}", token);
                        add_new_state(state.outs.0, p);
                        add_new_state(state.outs.1, p);
                    }
                }
                _ => {}
            }
        }

        self.current_states.drain();
        self.update_states_counts(&new_states);
        debug!(
            "  flush {}",
            self.state_params
                .iter()
                .map(|(k, v)| format!("p({})={}", k, v.1))
                .join(" ")
        );
        result = f32::max(result, self.add_states(&new_states, false));
        self.visited.drain();
        debug!("  p terminal={}", result);
        return result;
    }

    fn update_state(&mut self, i: usize, p: f32, count: bool) {
        let state = &self.nfa[i];
        debug!("    update {} p={} {}", state.kind, p, count);

        match state.kind {
            Kind::ExactQuantifier(n) => {
                let entry = self
                    .state_params
                    .entry(i)
                    .or_insert((Dist::ExactlyTimes(n), 0.0, 0));
                entry.1 = f32::max(entry.1, p);
                if count {
                    entry.2 += 1;
                }
            }
            _ => {
                self.current_states.insert(i);
                let entry = self
                    .state_params
                    .entry(i)
                    .or_insert((Dist::Constant(1.0), 0.0, 0));
                // let (prob, p, count) = entry;
                // *entry = (*prob, *p, *count + 1);
                entry.1 = f32::max(entry.1, p);
                if count {
                    entry.2 += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_state_init() {
        let nfa = vec![
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Split,
                },
                (Some(1), None),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('a'),
                },
                (Some(2), None),
            ),
        ];
        let mut state = NfaState::new(&nfa);
        state.init_state(Some(0), true);
        assert_eq!(state.current_states.len(), 1);
        assert_eq!(
            *state.state_params.get(&1).unwrap(),
            (Dist::Constant(1.0), 1.0, 0)
        );
        assert_eq!(state.visited.len(), 0);
    }

    #[test]
    fn test_state_add() {
        let nfa = vec![
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('a'),
                },
                (Some(1), None),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('b'),
                },
                (Some(2), None),
            ),
        ];
        let mut state = NfaState::new(&nfa);
        state.add_state(Some(0), true, 1.0);
        state.add_state(Some(0), true, 1.0);
        // TODO test update_states_counts
        // assert_eq!(
        //     *state.state_params.get(&0).unwrap(),
        //     (Dist::Constant(1.0), 1.0, 2)
        // );
        assert_eq!(state.visited.len(), 1);
    }

    // #[test]
    // fn test_state_add_tmp() {
    //     let nfa = vec![
    //         State::from(
    //             AstNode {
    //                 length: 1,
    //                 kind: Kind::Literal('a'),
    //             },
    //             (Some(1), None),
    //         ),
    //         State::from(
    //             AstNode {
    //                 length: 1,
    //                 kind: Kind::ExactQuantifier(2),
    //             },
    //             (Some(2), None),
    //         ),
    //     ];
    //     let mut state = NfaState::new(&nfa);
    //     state.add_state(Some(1), true);
    //     state.add_state(Some(1), true);
    //     assert_eq!(
    //         *state.state_params.get(&1).unwrap(),
    //         (Dist::Constant(1.0), 1.0, 2)
    //     );
    //     assert_eq!(state.visited.len(), 1);
    // }

    #[test]
    fn test_state_step() {
        let nfa = vec![
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('a'),
                },
                (Some(1), None),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('b'),
                },
                (Some(2), None),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('c'),
                },
                (None, None),
            ),
        ];
        let mut state = NfaState::new(&nfa);
        state.init_state(Some(0), true);
        assert_eq!(state.current_states.len(), 1);

        assert_eq!(state.step('a'), 0.0);
        assert_eq!(
            state.current_states.iter().collect::<Vec<&usize>>(),
            vec![&1]
        );

        assert_eq!(state.step('b'), 0.0);
        assert_eq!(
            state.current_states.iter().collect::<Vec<&usize>>(),
            vec![&2]
        );

        assert_eq!(state.step('x'), 0.0);
        assert_eq!(
            state.current_states.iter().collect::<Vec<&usize>>(),
            Vec::<&usize>::new()
        );

        assert_eq!(state.visited.len(), 0);
    }

    #[test]
    fn test_state_step_terminal() {
        let nfa = vec![
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Split,
                },
                (Some(1), None),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('a'),
                },
                (Some(2), None),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Terminal,
                },
                (None, None),
            ),
        ];
        let mut state = NfaState::new(&nfa);
        state.init_state(Some(0), true);

        assert_eq!(
            state.current_states.iter().collect::<Vec<&usize>>(),
            vec![&1]
        );
        assert_eq!(state.step('a'), 1.0);
        assert_eq!(state.visited.len(), 0);
    }

    #[test]
    fn test_state_probs() {
        let nfa = vec![
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Literal('a'),
                },
                (Some(1), None),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::ExactQuantifier(2),
                },
                (Some(0), Some(2)),
            ),
            State::from(
                AstNode {
                    length: 1,
                    kind: Kind::Terminal,
                },
                (None, None),
            ),
        ];
        let mut state = NfaState::new(&nfa);
        state.init_state(Some(0), true);
        assert_eq!(state.step('a'), 0.0);
        assert_eq!(state.step('a'), 1.0);
        let probs = state
            .state_params
            .keys()
            .sorted()
            .map(|k| state.state_params[k].1)
            .collect::<Vec<f32>>();
        assert_eq!(probs, vec![1.0, 0.0, 1.0]);
    }
}
