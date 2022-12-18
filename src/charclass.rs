use pest::iterators::Pair;

use crate::parser::Rule;

pub fn build_chars(pair: Pair<Rule>) -> Vec<char> {
    match pair.as_rule() {
        Rule::PosixClass | Rule::ShortClass => {
            let name = pair.as_str();
            let chars = match name {
                "[:digit:]" | "\\d" => vec!['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'],
                "[:space:]" | "\\s" => vec![' ', '\t', '\n', '\r', '\x0c', '\x0b'],
                _ => panic!("Unknown character class {}", name),
            };
            chars
        }
        Rule::CharacterClass => {
            let pairs = pair.into_inner();
            let chars: Vec<char> = pairs
                .flat_map(|p| match p.as_rule() {
                    Rule::PosixClass | Rule::ShortClass => build_chars(p),
                    _ => p.as_str().chars().collect(),
                })
                .collect();
            chars
        }
        _ => vec![],
    }
}
