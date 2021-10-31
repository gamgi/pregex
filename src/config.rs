use clap::{App, Arg, ArgMatches};

pub const PATTERN_OPTION: &str = "pattern";
pub const STRING_OPTION: &str = "string";

pub struct Config {
    pattern: String,
    string: String,
}

pub fn parse_options(options: ArgMatches) -> Result<Config, String> {
    let pattern = match options.value_of(PATTERN_OPTION) {
        Some(val) => Ok(String::from(val)),
        None => Err(String::from("Missing pattern. See --help.")),
    }?;
    let string = match options.value_of(STRING_OPTION) {
        Some(val) => Ok(String::from(val)),
        None => Err(String::from("Missing string to match. See --help.")),
    }?;

    return Ok(Config {
        pattern: pattern,
        string: string,
    });
}
