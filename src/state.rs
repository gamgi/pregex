use crate::ast::{AstNode, Kind};
use crate::nfa::State;
use std::collections::{HashMap, HashSet};

type ActiveState = (f32, u32); // (p, visits)

struct NfaState<'a> {
    nfa: &'a Vec<State>,
    visited: HashSet<usize>,
    pub current_states: HashMap<usize, ActiveState>,
}
const NEW_ACTIVE_STATE: ActiveState = (1.0, 0);

impl NfaState<'_> {
    pub fn new(nfa: &Vec<State>) -> NfaState {
        return NfaState {
            nfa: nfa,
            current_states: HashMap::new(),
            visited: HashSet::new(),
        };
    }

    pub fn add_state(&mut self, idx: Option<usize>, force: bool) {
        let i = if let Some(i) = idx { i } else { return };

        if let Some(state) = self.nfa.get(i) {
            let is_previously_visited = !self.visited.insert(i);
            if is_previously_visited && !force {
                debug!("    skip {}", self.nfa[i].kind.to_string());
                return;
            }
            // println!("{:?}", state.node.kind);
            match state.kind {
                Kind::Quantifier(_) | Kind::Start => {
                    // follow outs of quantifier
                    self.add_state(state.outs.0, force);
                    self.add_state(state.outs.1, force);
                    return;
                }
                _ => {
                    // add state
                    debug!("    add  {}", state.kind.to_string());
                    self.update_state(i, 1.0);
                }
            }
        }
    }

    fn update_state(&mut self, i: usize, _p: f32) {
        let entry = self.current_states.entry(i).or_insert(NEW_ACTIVE_STATE);
        let (p, count) = entry;
        *entry = (*p, *count + 1);
    }
}

#[cfg(test)]
mod test {
    use super::*;
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
        state.add_state(Some(0), true);
        state.add_state(Some(0), true);
        assert_eq!(*state.current_states.get(&0).unwrap(), (1.0, 2));
    }
}
