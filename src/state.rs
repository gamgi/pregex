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

impl NfaState<'_> {
    pub fn new(nfa: &Vec<State>) -> NfaState {
        return NfaState {
            nfa: nfa,
            current_states: HashSet::new(),
            visited: HashSet::new(),
            state_params: HashMap::new(),
        };
    }

    pub fn update_states_counts(&mut self, idxs: &Vec<Option<usize>>) {
        for idx in idxs.iter() {
            if let Some(i) = idx {
                self.update_state(*i, 0.0, true);
            }
        }
    }

    pub fn add_states(&mut self, idxs: Vec<Option<usize>>, force: bool) -> bool {
        idxs.into_iter()
            .map(|idx| self.add_state(idx, force, 1.0)) // TODO what is p
            .any(|is_terminal| is_terminal == true)
    }

    fn get_count(&self, idx: usize) -> u32 {
        if let Some(params) = self.state_params.get(&idx) {
            return params.2;
        }
        0
    }

    fn evaluate_state(&mut self, i: usize) -> (f32, f32) {
        // TODO should we get state_params.1 as p or pass it as arg?
        if let Some((dist, p0, n)) = self.state_params.get(&i) {
            return match dist {
                Dist::Constant(p) => (p * p0, p * p0),
                Dist::ExactlyTimes(match_n) => {
                    if n == match_n {
                        (0.0, *p0)
                    } else if n < match_n {
                        (*p0, 0.0)
                    } else {
                        (0.0, 0.0)
                    }
                }
            };
        }
        (0.0, 0.0) // TODO
    }

    pub fn add_state(&mut self, idx: Option<usize>, force: bool, p: f32) -> bool {
        // TODO return p instead of bool
        let i = if let Some(i) = idx { i } else { return false };

        if let Some(state) = self.nfa.get(i) {
            let is_previously_visited = !self.visited.insert(i);
            if is_previously_visited && !force {
                debug!("    skip {}", self.nfa[i].kind.to_string());
                return false;
            }

            match state.kind {
                Kind::Terminal => {
                    self.update_state(i, p, false);
                    return true;
                }
                Kind::Quantifier(_) | Kind::Start | Kind::Split => {
                    // follow outs of quantifier
                    let mut result = false;
                    result |= self.add_state(state.outs.0, force, p);
                    result |= self.add_state(state.outs.1, force, p);
                    return result;
                }
                Kind::ExactQuantifier(match_n) => {
                    let n = self.get_count(i);
                    let (p0, p1) = self.evaluate_state(i);
                    println!(
                        "  exq {} of {} {} {} ({} {})",
                        n, match_n, state.kind, p, p0, p1
                    );

                    let mut result = false;
                    result |= self.add_state(state.outs.1, true, p1);
                    result |= self.add_state(state.outs.0, true, p0);
                    return result;
                }
                _ => {
                    // add state
                    self.update_state(i, p, false);
                }
            }
        }
        false
    }

    pub fn init_state(&mut self, idx: Option<usize>, force: bool) {
        self.add_state(idx, force, 1.0);
        self.visited.drain();
    }

    pub fn step(&mut self, token: char) -> bool {
        println!("step {}", token);
        debug!("step {}", token);
        let mut new_states: Vec<Option<usize>> = Vec::new();
        let mut result = false;

        for i in self.current_states.iter() {
            let state = &self.nfa[*i];
            match state.kind {
                Kind::Terminal => result = true,
                Kind::Literal(match_token) => {
                    if match_token == token {
                        println!("  match {}", token);
                        new_states.push(state.outs.0);
                        new_states.push(state.outs.1);
                    }
                }
                _ => {}
            }
        }

        self.current_states.drain();
        self.update_states_counts(&new_states);
        println!("  flush {:?}", self.state_params);
        debug!("  flush");
        result |= self.add_states(new_states, false);
        self.visited.drain();
        println!("  {}", result);
        return result;
    }

    fn update_state(&mut self, i: usize, p: f32, count: bool) {
        debug!("  update {} {} {}", i, p, count);
        println!("  update {} {} {}", i, p, count);

        match self.nfa[i].kind {
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

        assert_eq!(state.step('a'), false);
        assert_eq!(
            state.current_states.iter().collect::<Vec<&usize>>(),
            vec![&1]
        );

        assert_eq!(state.step('b'), false);
        assert_eq!(
            state.current_states.iter().collect::<Vec<&usize>>(),
            vec![&2]
        );

        assert_eq!(state.step('x'), false);
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
        assert_eq!(state.step('a'), true);
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
        state.step('a');
        assert_eq!(state.step('a'), true);
        let probs = state
            .state_params
            .keys()
            .sorted()
            .map(|k| state.state_params[k].1)
            .collect::<Vec<f32>>();
        assert_eq!(probs, vec![1.0, 1.0, 1.0]);
    }
}
