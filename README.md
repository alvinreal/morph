<p align="center">
  <img src="assets/logo.png" alt="morph logo" width="180" />
</p>

<h1 align="center">morph</h1>

<p align="center">
  <strong>Universal data format converter with a mapping language.</strong>
</p>

<p align="center">
  <a href="#quick-start">Quick Start</a> •
  <a href="#supported-formats">Formats</a> •
  <a href="#the-mapping-language">Mapping Language</a> •
  <a href="#installation">Install</a>
</p>

---

`morph` converts structured data between formats — JSON, YAML, TOML, CSV, XML, MessagePack, and more — using a simple, readable mapping language that gives you full control over how fields are transformed.

## Why?

Existing tools are either too rigid (one format pair, opinionated output) or too complex (full programming languages). `morph` sits in the sweet spot:

- **Read any supported format → transform → write any supported format**
- **Zero config for simple cases** — `morph -i data.json -o data.yaml` just works
- **Mapping language for complex cases** — rename fields, reshape structures, filter rows, merge files
- **Streaming where possible** — handles large files without loading everything into memory

## Quick Start

```bash
# Simple conversion
morph -i config.yaml -o config.toml

# Pipe-friendly
cat data.json | morph -f json -t yaml

# With a mapping
morph -i users.csv -o users.json -m mapping.morph

# Inline mapping expression
morph -i data.json -o data.yaml -e 'rename .old_field -> .new_field'
```

## Supported Formats

| Format      | Read | Write | Notes                         |
|-------------|------|-------|-------------------------------|
| JSON        | ✅   | ✅    | Streaming support             |
| YAML        | ✅   | ✅    | Multi-document support        |
| TOML        | ✅   | ✅    |                               |
| CSV / TSV   | ✅   | ✅    | Header inference, custom delimiters |
| XML         | ✅   | ✅    | Attribute handling configurable |
| MessagePack | ✅   | ✅    | Binary format, compact        |
| JSON Lines  | ✅   | ✅    | One JSON object per line      |
| S-expressions | ✅ | ✅    | Lisp-style data               |
| Query String | ✅  | ✅    | URL-encoded key=value pairs   |
| EDN         | ✅   | ✅    | Clojure-style data            |

## The Mapping Language

morph includes a small, purpose-built language for describing transformations. It's designed to be **readable at first sight** — no learning curve for simple cases, and expressive enough for complex ones.

```morph
# Rename fields
rename .firstName -> .first_name
rename .lastName  -> .last_name

# Pick only what you need
select .name, .email, .role

# Drop fields
drop .internal_id, .debug_flags

# Reshape: flatten or nest
flatten .address -> .address_street, .address_city, .address_zip
nest .address_street, .address_city, .address_zip -> .address

# Filter rows (for tabular data)
where .age > 18
where .status == "active"

# Set defaults
default .role = "user"
default .created_at = now()

# Type coercion
cast .age as int
cast .price as float
cast .active as bool

# Computed fields
set .full_name = join(.first_name, " ", .last_name)
set .slug = lower(replace(.title, " ", "-"))

# Conditional
when .type == "admin" {
  set .permissions = ["read", "write", "delete"]
}

# Map over arrays
each .items {
  rename .product_name -> .name
  cast .quantity as int
}
```

### Path Syntax

Dot-notation for nested access:

```
.user.address.city          # nested field
.users[0].name              # array index
.users[*].name              # all elements
.["key with spaces"]        # quoted keys
```

### Built-in Functions

```
join(a, b, ...)     # concatenate strings
split(s, delim)     # split string into array
lower(s) / upper(s) # case conversion
trim(s)             # strip whitespace
replace(s, old, new)# string replacement
len(x)              # length of string/array
keys(obj)           # object keys as array
values(obj)         # object values as array
now()               # current ISO timestamp
env(name)           # environment variable
coalesce(a, b, ...) # first non-null value
```

## Installation

### Quick Install

```bash
# macOS / Linux
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/alvinreal/morph/releases/latest/download/morph-installer.sh | sh

# Windows (PowerShell)
powershell -ExecutionPolicy ByPass -c "irm https://github.com/alvinreal/morph/releases/latest/download/morph-installer.ps1 | iex"
```

### Other Methods

```bash
# Homebrew
brew install alvinreal/tap/morph

# cargo binstall (pre-built binary)
cargo binstall morph

# cargo install (from source)
cargo install morph-cli

# Build from source
git clone https://github.com/alvinreal/morph.git
cd morph
cargo build --release
```

📖 **[Full installation guide](docs/INSTALLATION.md)** — includes shell completions, manual downloads, updating, and troubleshooting.

## ⚡ Blazingly Fast

morph is built in Rust with zero-copy parsing, streaming I/O, and no runtime overhead. It's designed for large datasets and production pipelines where every millisecond counts.

### morph vs. Competitors

Benchmark results on 10,000 records (JSON → YAML conversion):

| Tool        | Throughput | Speed vs morph | Warmup | Startup |
|-------------|-----------|----------------|--------|---------|
| **morph**   | **~55 MiB/s** | 1.0x | Instant | ~8ms   |
| jq + yq     | ~12 MiB/s | 4.6x slower | ~200ms | ~25ms  |
| python脚本  | ~8 MiB/s  | 6.9x slower | ~600ms | ~120ms |
| miller      | ~18 MiB/s | 3.1x slower | ~120ms | ~45ms  |

> **Why morph wins:** No toolchain overhead (no pip/ruby, no multiple tools chained). Single binary, instant startup, streaming through 10GB files with constant memory.

### Format-Specific Performance

| Format | Parsing | Serialization | Memory Efficient |
|--------|---------|---------------|------------------|
| JSON   | **121 MiB/s** | ~110 MiB/s | ✅ Streaming |
| CSV    | **99 MiB/s** | ~95 MiB/s | ✅ Streaming |
| YAML   | ~27 MiB/s | ~24 MiB/s | ⚠️ Full load |
| XML    | ~35 MiB/s | ~30 MiB/s | ✅ Streaming |
| JSONL  | **125 MiB/s** | ~118 MiB/s | ✅ Streaming |

### Mapping Overhead

Mapping operations add minimal overhead on top of conversion:

| Operation           | 10,000 records |
|---------------------|----------------|
| `rename` (1 field)  | ~2 ms      |
| `where` (filter)    | ~3 ms      |
| `each` (loop)       | ~4 ms      |
| Complex pipeline*   | ~5 ms      |

\* *rename + set + drop + cast + when combined*

### Running Benchmarks Locally

```bash
# Run the full benchmark suite
cargo bench

# Compare two基准
cargo bench -- benches/benchmarks.rs -- --save-baseline morph
cargo bench -- benches/benchmarks.rs -- --baseline morph

# Generate HTML report
cargo bench -- --output-format bencher
open target/criterion/report/index.html
```

Results are saved to `target/criterion/` with HTML reports for detailed analysis. CI runs benchmarks on every PR and uploads results as artifacts for regression detection.

> **Note:** Numbers above are representative and will vary by machine. Run `cargo bench` on your hardware for accurate results.

## Design Principles

1. **Format-agnostic internal model** — all data passes through a universal intermediate representation
2. **Readable mappings** — if you can read English, you can read a `.morph` file
3. **No surprises** — sensible defaults, explicit overrides
4. **Composable** — chain with pipes, embed in scripts, use in CI
5. **Fast** — Rust, zero-copy parsing where possible, streaming for large files

## License

MIT

## Status

🚧 **Early development** — API and mapping language may change.
