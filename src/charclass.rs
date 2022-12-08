use pest::iterators::Pair;

use crate::parser::Rule;

pub fn build_chars(pair: Pair<Rule>) -> Vec<char> {
    match pair.as_rule() {
        Rule::ShortClass => {
            let name = pair.as_str();
            let chars = match name {
                _ => panic!("Unknown character class {}", name),
            };
            chars
        }
        Rule::CharacterClass => {
            let pairs = pair.into_inner();
            let chars: Vec<char> = pairs.map(|r| r.as_str().chars().next().unwrap()).collect();
            chars
        }
        _ => vec![],
    }
}
