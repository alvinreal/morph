# Contributing to morph

Thanks for considering contributing to morph! Here's how to get started.

## Development Setup

```bash
# Clone the repo
git clone https://github.com/alvinreal/morph.git
cd morph

# Build
cargo build

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run -- -i test.json -o test.yaml
```

## Project Structure

```
src/
├── main.rs          # CLI entry point
├── lib.rs           # Library root
├── value.rs         # Universal Value type
├── error.rs         # Error types
├── cli.rs           # Argument parsing
├── formats/         # Format readers/writers
│   ├── mod.rs       # Registry & detection
│   ├── json.rs
│   ├── yaml.rs
│   └── ...
├── mapping/         # Mapping language
│   ├── lexer.rs     # Tokenizer
│   ├── parser.rs    # AST construction
│   ├── ast.rs       # AST types
│   ├── eval.rs      # Evaluator
│   └── functions.rs # Built-in functions
└── tests/
```

## Guidelines

- **Tests:** Add tests for any new functionality
- **Formatting:** Run `cargo fmt` before committing
- **Linting:** Run `cargo clippy` and address warnings
- **Docs:** Update relevant docs for user-facing changes

## Adding a New Format

1. Create `src/formats/<name>.rs`
2. Implement the `Reader` and `Writer` traits
3. Register in `src/formats/mod.rs`
4. Add file extension mappings
5. Add round-trip tests in `tests/formats/`
6. Update the format table in README.md and PRD.md

## Adding a New Function

1. Add the function implementation in `src/mapping/functions.rs`
2. Register it in the function lookup table
3. Add tests in `tests/mapping/`
4. Document in `docs/MAPPING_LANGUAGE.md`

## Pull Requests

- Keep PRs focused — one feature or fix per PR
- Include tests
- Update docs if needed
- Describe what and why in the PR description

## Code of Conduct

Be kind, be constructive, be respectful. We're all here to build something useful.
