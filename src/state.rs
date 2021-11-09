use crate::ast::{AstNode, Kind};
use crate::nfa::State;
// use crate::nfa::Outs;
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

    pub fn add_states(&mut self, idxs: Vec<Option<usize>>, force: bool) {
        for i in idxs.into_iter() {
            self.add_state(i, force);
        }
    }

    pub fn add_state(&mut self, idx: Option<usize>, force: bool) {
        let i = if let Some(i) = idx { i } else { return };

        if let Some(state) = self.nfa.get(i) {
            let is_previously_visited = !self.visited.insert(i);
            if is_previously_visited && !force {
                debug!("    skip {}", self.nfa[i].kind.to_string());
                return;
            }

            match state.kind {
                Kind::Quantifier(_) | Kind::Start => {
                    // follow outs of quantifier
                    self.add_state(state.outs.0, force);
                    self.add_state(state.outs.1, force);
                    return;
                }
                _ => {
                    // add state
                    // debug!("    add  {}", state.node.to_string());
                    self.update_state(i, 1.0);
                }
            }
        }
    }

    pub fn init_state(&mut self, idx: Option<usize>, force: bool) {
        // let state = &self.nfa[*i];
        self.add_state(idx, force);
        // self.visited.drain();
        // TODO
    }

    // pub fn step(&mut self, token: char) -> bool {
    // pub fn step(&mut self, token: char) -> HashMap<usize, ActiveState> {
    pub fn step(&mut self, match_token: char) -> bool {
        // let mut new_states = HashMap::new();
        // for (i, (_, _)) in self.current_states.iter_mut() {
        // let foo = self.current_states.copy();
        let mut new: Vec<Option<usize>> = Vec::new();
        // for (i, state) in self.current_states.iter() {
        for (i, (token, _, _)) in self.current_states.iter() {
            let state = &self.nfa[*i];
            match state.kind {
                // if let Kind::Literal(c) == token {
                Kind::Quantifier(c) => {
                    new.push(state.outs.0);
                    new.push(state.outs.1);
                }
                Kind::Literal(c) => {
                    // if *c == token {
                    if c == match_token {
                        // let outs = self.nfa[*i].outs;

                        new.push(Some(*i));
                        // new.push(outs.1);
                    }
                },
                _ => {}
            }
        }
        self.add_states(new, true);
        return false;
        // for (i, state) in self.current_states.iter() {
        //     // current_states: HashMap<usize, ActiveState>,
        //     // let state = self.nfa[*i];
        //     // if let Kind::Terminal {

        //     // }
        //     if state.0 == token {
        //         // *state =
        //         self.add_state(Some(0), true);
        //         // self.add_state(state.outs.0, true);
        //     }
        // }
        // return new_states
    }

    fn update_state(&mut self, i: usize, _p: f32) {
        if let Kind::Literal(c) = self.nfa[i].kind {
            // let tokend = &self.nfa[*i];
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
        assert_eq!(*state.current_states.get(&0).unwrap(), ('a', 1.0, 2));
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
    }
}
