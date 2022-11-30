use {clap::Parser, std::path::PathBuf};

#[derive(Parser, Debug, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Config {
    /// Verbosity
    #[clap(short, parse(from_occurrences))]
    pub verbosity: usize,

    /// Input file or - for stdin
    #[clap(short, long, value_name = "FILE")]
    pub input_file: Option<String>,

    /// Regex pattern
    #[clap(required = true)]
    pub pattern: String,

    /// String to match
    #[clap(
        required_unless_present("input-file"),
        conflicts_with("input-file"),
        value_name = "STRING"
    )]
    pub input_string: Option<String>,
}
