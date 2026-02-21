use std::process;

fn main() {
    let cli = morph::cli::Cli::parse_args();
    if let Err(e) = morph::cli::run(&cli) {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}
