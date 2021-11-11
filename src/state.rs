#![allow(dead_code, unused_variables)]
use crate::ast::{AstNode, Kind};
use crate::nfa::State;
use std::collections::{HashMap, HashSet};

type ActiveState = (char, f32, u32); // (c, p, visits)

struct NfaState<'a> {
    nfa: &'a Vec<State>,
    visited: HashSet<usize>,
    pub current_states: HashMap<usize, ActiveState>,
}

impl NfaState<'_> {
    pub fn new(nfa: &Vec<State>) -> NfaState {
        return NfaState {
            nfa: nfa,
            current_states: HashMap::new(),
            visited: HashSet::new(),
        };
    }

    pub fn add_states(&mut self, idxs: Vec<Option<usize>>, force: bool) -> bool {
        idxs.into_iter()
            .map(|idx| self.add_state(idx, force))
            .any(|is_terminal| is_terminal == true)
    }

    pub fn add_state(&mut self, idx: Option<usize>, force: bool) -> bool {
        let i = if let Some(i) = idx { i } else { return false };

        if let Some(state) = self.nfa.get(i) {
            let is_previously_visited = !self.visited.insert(i);
            if is_previously_visited && !force {
                debug!("    skip {}", self.nfa[i].kind.to_string());
                return false;
            }

            match state.kind {
                Kind::Terminal => return true,
                Kind::Quantifier(_) | Kind::Start | Kind::Split => {
                    // follow outs of quantifier
                    self.add_state(state.outs.0, force);
                    self.add_state(state.outs.1, force);
                }
                _ => {
                    // add state
                    debug!("    add  {}", state.kind.to_string());
                    self.update_state(i, 1.0);
                }
            }
        }
        false
    }

    pub fn init_state(&mut self, idx: Option<usize>, force: bool) {
        self.add_state(idx, force);
        self.visited.drain();
    }

    pub fn step(&mut self, match_token: char) -> bool {
        let mut new_states: Vec<Option<usize>> = Vec::new();
        for (i, _) in self.current_states.iter() {
            let state = &self.nfa[*i];
            match state.kind {
                Kind::Terminal => return true,
                Kind::Quantifier(_) | Kind::Split => {
                    new_states.push(state.outs.0);
                    new_states.push(state.outs.1);
                }
                Kind::Literal(c) => {
                    if c == match_token {
                        new_states.push(state.outs.0);
                        new_states.push(state.outs.1);
                    }
                }
                _ => {}
            }
        }
        self.current_states.drain();
        let result = self.add_states(new_states, true);
        self.visited.drain();
        return result;
    }

    fn update_state(&mut self, i: usize, _p: f32) {
        if let Kind::Literal(c) = self.nfa[i].kind {
            let entry = self.current_states.entry(i).or_insert((c, 1.0, 0));
            let (c, p, count) = entry;
            *entry = (*c, *p, *count + 1);
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
        assert_eq!(*state.current_states.get(&1).unwrap(), ('a', 1.0, 1));
        assert_eq!(state.visited.len(), 0);
    }

    #[test]
    fn test_add_state() {
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
        state.add_state(Some(0), true);
        state.add_state(Some(0), true);
        assert_eq!(*state.current_states.get(&0).unwrap(), ('a', 1.0, 2));
        assert_eq!(state.visited.len(), 1);
    }

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
            state.current_states.keys().collect::<Vec<&usize>>(),
            vec![&1]
        );

        assert_eq!(state.step('b'), false);
        assert_eq!(
            state.current_states.keys().collect::<Vec<&usize>>(),
            vec![&2]
        );

        assert_eq!(state.step('x'), false);
        assert_eq!(
            state.current_states.keys().collect::<Vec<&usize>>(),
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
            state.current_states.keys().collect::<Vec<&usize>>(),
            vec![&1]
        );
        assert_eq!(state.step('a'), true);
        assert_eq!(state.visited.len(), 0);
    }
}
