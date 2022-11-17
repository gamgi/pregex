use clap::{App, Arg, ArgMatches};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const NORMALIZE_OPTION: &str = "normalize";
const PATTERN_OPTION: &str = "pattern";
const INPUT_STRING_OPTION: &str = "string";
const INPUT_FILE_OPTION: &str = "input";

pub struct Config {
    pub normalize: bool,
    pub pattern: String,
    pub input_string: String,
    pub input_file: Option<String>,
    pub verbosity: usize,
}

pub fn options() -> App<'static> {
    App::new("Pregex")
        .version(VERSION)
        .arg(
            Arg::new(NORMALIZE_OPTION)
                .short('n')
                .long(NORMALIZE_OPTION)
                .takes_value(false)
                .help("Normalize results"),
        )
        .arg(
            Arg::new("v")
                .short('v')
                .multiple_occurrences(true)
                .help("Sets the level of verbosity"),
        )
        .arg(
            Arg::new(INPUT_FILE_OPTION)
                .short('f')
                .long(INPUT_FILE_OPTION)
                .value_name("NAME")
                .takes_value(true)
                .help("Input file or - for stdin"),
        )
        .arg(
            Arg::new(PATTERN_OPTION)
                .value_name("PATTERN")
                .help("Regex pattern")
                .index(1),
        )
        .arg(
            Arg::new(INPUT_STRING_OPTION)
                .value_name("STRING")
                .help("String to match against")
                .index(2)
                .required_unless_present(INPUT_FILE_OPTION),
        )
}

pub fn parse_options(matches: ArgMatches) -> crate::Result<Config> {
    let normalize = matches.is_present(NORMALIZE_OPTION);
    let input_file = matches.value_of(INPUT_FILE_OPTION).map(str::to_string);
    let input_string = matches
        .value_of(INPUT_STRING_OPTION)
        .unwrap_or_default()
        .to_string();
    let pattern = matches.value_of(PATTERN_OPTION).unwrap().to_string();
    let verbosity = match matches.occurrences_of("v") {
        0 => 0,
        1 => 1,
        _ => 2,
    };

    Ok(Config {
        normalize,
        pattern,
        input_string,
        input_file,
        verbosity,
    })
}
