use clap::Parser;
use std::fmt;
use std::path::PathBuf;

/// Supported data formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Json,
    Yaml,
    Toml,
    Csv,
}

impl Format {
    /// All known formats and their display names.
    pub fn all() -> &'static [(Format, &'static str, &'static [&'static str])] {
        &[
            (Format::Json, "JSON", &["json"]),
            (Format::Yaml, "YAML", &["yaml", "yml"]),
            (Format::Toml, "TOML", &["toml"]),
            (Format::Csv, "CSV", &["csv"]),
        ]
    }

    /// Detect format from a file extension.
    pub fn from_extension(ext: &str) -> Option<Format> {
        match ext.to_lowercase().as_str() {
            "json" => Some(Format::Json),
            "yaml" | "yml" => Some(Format::Yaml),
            "toml" => Some(Format::Toml),
            "csv" => Some(Format::Csv),
            _ => None,
        }
    }

    /// Detect format from a file path (by extension).
    pub fn from_path(path: &std::path::Path) -> Option<Format> {
        path.extension()
            .and_then(|e| e.to_str())
            .and_then(Format::from_extension)
    }

    /// Parse a format name string.
    pub fn from_name(name: &str) -> Option<Format> {
        match name.to_lowercase().as_str() {
            "json" => Some(Format::Json),
            "yaml" | "yml" => Some(Format::Yaml),
            "toml" => Some(Format::Toml),
            "csv" => Some(Format::Csv),
            _ => None,
        }
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Format::Json => write!(f, "json"),
            Format::Yaml => write!(f, "yaml"),
            Format::Toml => write!(f, "toml"),
            Format::Csv => write!(f, "csv"),
        }
    }
}

/// morph — a universal data format converter
#[derive(Parser, Debug)]
#[command(name = "morph", version, about = "Convert between data formats")]
pub struct Cli {
    /// Input file (reads from stdin if not specified)
    #[arg(short = 'i', long = "input")]
    pub input: Option<PathBuf>,

    /// Output file (writes to stdout if not specified)
    #[arg(short = 'o', long = "output")]
    pub output: Option<PathBuf>,

    /// Input format (auto-detected from file extension if not specified)
    #[arg(short = 'f', long = "from")]
    pub from: Option<String>,

    /// Output format (auto-detected from file extension if not specified)
    #[arg(short = 't', long = "to")]
    pub to: Option<String>,

    /// Pretty-print output (default for TTY)
    #[arg(long = "pretty", conflicts_with = "compact")]
    pub pretty: bool,

    /// Compact output (default for pipes)
    #[arg(long = "compact", conflicts_with = "pretty")]
    pub compact: bool,

    /// Indentation width (spaces)
    #[arg(long = "indent")]
    pub indent: Option<usize>,

    /// List supported formats
    #[arg(long = "formats")]
    pub formats: bool,

    /// Apply a mapping file (.morph)
    #[arg(short = 'm', long = "mapping")]
    pub mapping: Option<PathBuf>,

    /// Inline mapping expression (can be repeated; applied in order after -m)
    #[arg(short = 'e', long = "expr", action = clap::ArgAction::Append)]
    pub expr: Vec<String>,

    /// Parse and validate the mapping without executing
    #[arg(long = "dry-run")]
    pub dry_run: bool,
}

impl Cli {
    /// Parse arguments from the command line.
    pub fn parse_args() -> Self {
        Cli::parse()
    }

    /// Resolve the input format from the `--from` flag or the input file extension.
    pub fn resolve_input_format(&self) -> crate::error::Result<Format> {
        if let Some(ref name) = self.from {
            return Format::from_name(name).ok_or_else(|| {
                crate::error::MorphError::cli(format!("unknown input format: '{name}'"))
            });
        }
        if let Some(ref path) = self.input {
            return Format::from_path(path).ok_or_else(|| {
                crate::error::MorphError::cli(format!(
                    "cannot detect format from '{}', use -f/--from to specify",
                    path.display()
                ))
            });
        }
        Err(crate::error::MorphError::cli(
            "reading from stdin requires -f/--from to specify input format",
        ))
    }

    /// Resolve the output format from the `--to` flag or the output file extension.
    pub fn resolve_output_format(&self) -> crate::error::Result<Format> {
        if let Some(ref name) = self.to {
            return Format::from_name(name).ok_or_else(|| {
                crate::error::MorphError::cli(format!("unknown output format: '{name}'"))
            });
        }
        if let Some(ref path) = self.output {
            return Format::from_path(path).ok_or_else(|| {
                crate::error::MorphError::cli(format!(
                    "cannot detect format from '{}', use -t/--to to specify",
                    path.display()
                ))
            });
        }
        Err(crate::error::MorphError::cli(
            "writing to stdout requires -t/--to to specify output format",
        ))
    }
}

/// Read input data based on the CLI args.
pub fn read_input(cli: &Cli) -> crate::error::Result<String> {
    match &cli.input {
        Some(path) => std::fs::read_to_string(path).map_err(|e| {
            crate::error::MorphError::Io(std::io::Error::new(
                e.kind(),
                format!("{}: {e}", path.display()),
            ))
        }),
        None => {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
    }
}

/// Parse input string according to format.
pub fn parse_input(input: &str, format: Format) -> crate::error::Result<crate::value::Value> {
    match format {
        Format::Json => crate::formats::json::from_str(input),
        Format::Yaml => crate::formats::yaml::from_str(input),
        Format::Toml => crate::formats::toml::from_str(input),
        Format::Csv => crate::formats::csv::from_str(input),
    }
}

/// Serialize a value to string according to format and pretty-print preference.
pub fn serialize_output(
    value: &crate::value::Value,
    format: Format,
    pretty: bool,
) -> crate::error::Result<String> {
    match format {
        Format::Json => {
            if pretty {
                crate::formats::json::to_string_pretty(value)
            } else {
                crate::formats::json::to_string(value)
            }
        }
        Format::Yaml => crate::formats::yaml::to_string(value),
        Format::Toml => crate::formats::toml::to_string(value),
        Format::Csv => crate::formats::csv::to_string(value),
    }
}

/// Write output string to file or stdout.
pub fn write_output(cli: &Cli, output: &str) -> crate::error::Result<()> {
    match &cli.output {
        Some(path) => std::fs::write(path, output).map_err(|e| {
            crate::error::MorphError::Io(std::io::Error::new(
                e.kind(),
                format!("{}: {e}", path.display()),
            ))
        }),
        None => {
            print!("{output}");
            Ok(())
        }
    }
}

/// Build a combined mapping program from -m and -e flags.
/// Returns Ok(None) if no mapping flags were given.
pub fn build_mapping_program(
    cli: &Cli,
) -> crate::error::Result<Option<crate::mapping::ast::Program>> {
    let has_mapping = cli.mapping.is_some();
    let has_exprs = !cli.expr.is_empty();

    if !has_mapping && !has_exprs {
        return Ok(None);
    }

    let mut all_statements = Vec::new();

    // Load mapping file first
    if let Some(ref path) = cli.mapping {
        let source = std::fs::read_to_string(path).map_err(|e| {
            crate::error::MorphError::Io(std::io::Error::new(
                e.kind(),
                format!("{}: {e}", path.display()),
            ))
        })?;
        let program = crate::mapping::parser::parse_str(&source)?;
        all_statements.extend(program.statements);
    }

    // Then append inline expressions
    for expr_str in &cli.expr {
        let program = crate::mapping::parser::parse_str(expr_str)?;
        all_statements.extend(program.statements);
    }

    Ok(Some(crate::mapping::ast::Program {
        statements: all_statements,
    }))
}

/// Run the full pipeline based on CLI args.
pub fn run(cli: &Cli) -> crate::error::Result<()> {
    if cli.formats {
        println!("Supported formats:");
        for (_, name, extensions) in Format::all() {
            println!("  {name} (extensions: {})", extensions.join(", "));
        }
        return Ok(());
    }

    // Build mapping program (if any flags given)
    let mapping_program = build_mapping_program(cli)?;

    // --dry-run: validate mapping and exit
    if cli.dry_run {
        match &mapping_program {
            Some(_) => {
                println!("mapping valid");
                return Ok(());
            }
            None => {
                println!("mapping valid");
                return Ok(());
            }
        }
    }

    let in_fmt = cli.resolve_input_format()?;
    let out_fmt = cli.resolve_output_format()?;

    let input_data = read_input(cli)?;
    let value = parse_input(&input_data, in_fmt)?;

    // Apply mapping if present
    let value = match mapping_program {
        Some(ref program) => crate::mapping::eval::eval(program, &value)?,
        None => value,
    };

    // Determine pretty-printing: explicit flags > default based on TTY
    let pretty = if cli.pretty {
        true
    } else if cli.compact {
        false
    } else {
        // Default: pretty for file output or TTY stdout, compact for piped
        cli.output.is_some() || atty_stdout()
    };

    let output_data = serialize_output(&value, out_fmt, pretty)?;
    write_output(cli, &output_data)?;

    Ok(())
}

/// Check if stdout is a TTY (best-effort).
fn atty_stdout() -> bool {
    use std::io::IsTerminal;
    std::io::stdout().is_terminal()
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Format detection ---------------------------------------------------

    #[test]
    fn format_from_extension_json() {
        assert_eq!(Format::from_extension("json"), Some(Format::Json));
    }

    #[test]
    fn format_from_extension_yaml() {
        assert_eq!(Format::from_extension("yaml"), Some(Format::Yaml));
        assert_eq!(Format::from_extension("yml"), Some(Format::Yaml));
    }

    #[test]
    fn format_from_extension_toml() {
        assert_eq!(Format::from_extension("toml"), Some(Format::Toml));
    }

    #[test]
    fn format_from_extension_csv() {
        assert_eq!(Format::from_extension("csv"), Some(Format::Csv));
    }

    #[test]
    fn format_from_extension_unknown() {
        assert_eq!(Format::from_extension("xyz"), None);
    }

    #[test]
    fn format_from_path_json() {
        let p = PathBuf::from("data.json");
        assert_eq!(Format::from_path(&p), Some(Format::Json));
    }

    #[test]
    fn format_from_path_yaml() {
        let p = PathBuf::from("config.yaml");
        assert_eq!(Format::from_path(&p), Some(Format::Yaml));

        let p2 = PathBuf::from("config.yml");
        assert_eq!(Format::from_path(&p2), Some(Format::Yaml));
    }

    #[test]
    fn format_from_name() {
        assert_eq!(Format::from_name("json"), Some(Format::Json));
        assert_eq!(Format::from_name("JSON"), Some(Format::Json));
        assert_eq!(Format::from_name("yaml"), Some(Format::Yaml));
        assert_eq!(Format::from_name("toml"), Some(Format::Toml));
        assert_eq!(Format::from_name("csv"), Some(Format::Csv));
        assert_eq!(Format::from_name("xml"), None);
    }

    // -- Arg parsing --------------------------------------------------------

    #[test]
    fn arg_parsing_basic() {
        let cli = Cli::try_parse_from(["morph", "-i", "input.json", "-o", "output.yaml"]).unwrap();
        assert_eq!(cli.input, Some(PathBuf::from("input.json")));
        assert_eq!(cli.output, Some(PathBuf::from("output.yaml")));
    }

    #[test]
    fn arg_parsing_format_flags() {
        let cli = Cli::try_parse_from(["morph", "-f", "json", "-t", "toml"]).unwrap();
        assert_eq!(cli.from, Some("json".to_string()));
        assert_eq!(cli.to, Some("toml".to_string()));
    }

    #[test]
    fn arg_parsing_pretty() {
        let cli = Cli::try_parse_from(["morph", "--pretty", "-f", "json", "-t", "json"]).unwrap();
        assert!(cli.pretty);
        assert!(!cli.compact);
    }

    #[test]
    fn arg_parsing_compact() {
        let cli = Cli::try_parse_from(["morph", "--compact", "-f", "json", "-t", "json"]).unwrap();
        assert!(cli.compact);
        assert!(!cli.pretty);
    }

    #[test]
    fn arg_parsing_formats_flag() {
        let cli = Cli::try_parse_from(["morph", "--formats"]).unwrap();
        assert!(cli.formats);
    }

    // -- Format resolution --------------------------------------------------

    #[test]
    fn resolve_input_format_from_flag() {
        let cli = Cli::try_parse_from(["morph", "-f", "json", "-t", "yaml"]).unwrap();
        assert_eq!(cli.resolve_input_format().unwrap(), Format::Json);
    }

    #[test]
    fn resolve_input_format_from_extension() {
        let cli = Cli::try_parse_from(["morph", "-i", "data.yaml", "-t", "json"]).unwrap();
        assert_eq!(cli.resolve_input_format().unwrap(), Format::Yaml);
    }

    #[test]
    fn resolve_input_format_flag_overrides_extension() {
        let cli =
            Cli::try_parse_from(["morph", "-i", "data.yaml", "-f", "json", "-t", "toml"]).unwrap();
        assert_eq!(cli.resolve_input_format().unwrap(), Format::Json);
    }

    #[test]
    fn resolve_input_format_stdin_requires_flag() {
        let cli = Cli::try_parse_from(["morph", "-t", "json"]).unwrap();
        assert!(cli.resolve_input_format().is_err());
    }

    #[test]
    fn resolve_output_format_from_flag() {
        let cli = Cli::try_parse_from(["morph", "-f", "json", "-t", "yaml"]).unwrap();
        assert_eq!(cli.resolve_output_format().unwrap(), Format::Yaml);
    }

    #[test]
    fn resolve_output_format_from_extension() {
        let cli = Cli::try_parse_from(["morph", "-f", "json", "-o", "out.toml"]).unwrap();
        assert_eq!(cli.resolve_output_format().unwrap(), Format::Toml);
    }

    #[test]
    fn resolve_unknown_extension_error() {
        let cli = Cli::try_parse_from(["morph", "-i", "data.xyz", "-t", "json"]).unwrap();
        let err = cli.resolve_input_format().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("cannot detect format"), "msg: {msg}");
        assert!(msg.contains("-f") || msg.contains("--from"), "msg: {msg}");
    }

    // -- Parse / serialize pipeline -----------------------------------------

    #[test]
    fn parse_and_serialize_json_to_yaml() {
        let input = r#"{"name": "Alice", "age": 30}"#;
        let val = parse_input(input, Format::Json).unwrap();
        let output = serialize_output(&val, Format::Yaml, false).unwrap();
        assert!(output.contains("name:"));
        assert!(output.contains("Alice"));
    }

    #[test]
    fn parse_and_serialize_yaml_to_json() {
        let input = "name: Alice\nage: 30\n";
        let val = parse_input(input, Format::Yaml).unwrap();
        let output = serialize_output(&val, Format::Json, false).unwrap();
        assert!(output.contains("\"name\""));
        assert!(output.contains("\"Alice\""));
    }

    #[test]
    fn format_display() {
        assert_eq!(Format::Json.to_string(), "json");
        assert_eq!(Format::Yaml.to_string(), "yaml");
        assert_eq!(Format::Toml.to_string(), "toml");
        assert_eq!(Format::Csv.to_string(), "csv");
    }

    // -- Mapping CLI flags --------------------------------------------------

    #[test]
    fn arg_parsing_mapping_file() {
        let cli = Cli::try_parse_from([
            "morph",
            "-i",
            "in.json",
            "-o",
            "out.json",
            "-m",
            "transform.morph",
        ])
        .unwrap();
        assert_eq!(cli.mapping, Some(PathBuf::from("transform.morph")));
    }

    #[test]
    fn arg_parsing_single_expr() {
        let cli =
            Cli::try_parse_from(["morph", "-f", "json", "-t", "json", "-e", "rename .x -> .y"])
                .unwrap();
        assert_eq!(cli.expr, vec!["rename .x -> .y"]);
    }

    #[test]
    fn arg_parsing_multiple_expr() {
        let cli = Cli::try_parse_from([
            "morph",
            "-f",
            "json",
            "-t",
            "json",
            "-e",
            "rename .x -> .y",
            "-e",
            "drop .z",
        ])
        .unwrap();
        assert_eq!(cli.expr, vec!["rename .x -> .y", "drop .z"]);
    }

    #[test]
    fn arg_parsing_dry_run() {
        let cli = Cli::try_parse_from([
            "morph",
            "--dry-run",
            "-e",
            "drop .x",
            "-f",
            "json",
            "-t",
            "json",
        ])
        .unwrap();
        assert!(cli.dry_run);
    }

    #[test]
    fn arg_parsing_mapping_and_expr_combined() {
        let cli = Cli::try_parse_from([
            "morph",
            "-m",
            "base.morph",
            "-e",
            "drop .extra",
            "-f",
            "json",
            "-t",
            "yaml",
        ])
        .unwrap();
        assert_eq!(cli.mapping, Some(PathBuf::from("base.morph")));
        assert_eq!(cli.expr, vec!["drop .extra"]);
    }

    #[test]
    fn no_mapping_flags_returns_none() {
        let cli = Cli::try_parse_from(["morph", "-f", "json", "-t", "yaml"]).unwrap();
        let program = build_mapping_program(&cli).unwrap();
        assert!(program.is_none());
    }

    #[test]
    fn build_mapping_from_expr() {
        let cli = Cli::try_parse_from([
            "morph",
            "-f",
            "json",
            "-t",
            "json",
            "-e",
            "rename .old -> .new",
        ])
        .unwrap();
        let program = build_mapping_program(&cli).unwrap();
        assert!(program.is_some());
        assert_eq!(program.unwrap().statements.len(), 1);
    }

    #[test]
    fn build_mapping_multiple_exprs_in_order() {
        let cli = Cli::try_parse_from([
            "morph",
            "-f",
            "json",
            "-t",
            "json",
            "-e",
            "rename .a -> .b",
            "-e",
            "drop .c",
        ])
        .unwrap();
        let program = build_mapping_program(&cli).unwrap();
        assert!(program.is_some());
        assert_eq!(program.unwrap().statements.len(), 2);
    }

    #[test]
    fn build_mapping_invalid_expr_returns_error() {
        let cli = Cli::try_parse_from([
            "morph",
            "-f",
            "json",
            "-t",
            "json",
            "-e",
            "invalid!!!syntax",
        ])
        .unwrap();
        let result = build_mapping_program(&cli);
        assert!(result.is_err());
    }

    #[test]
    fn build_mapping_nonexistent_file_returns_error() {
        let cli = Cli::try_parse_from([
            "morph",
            "-f",
            "json",
            "-t",
            "json",
            "-m",
            "/nonexistent/path/transform.morph",
        ])
        .unwrap();
        let result = build_mapping_program(&cli);
        assert!(result.is_err());
    }
}
