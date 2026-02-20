# morph — Product Requirements Document

## Vision

`morph` is a universal CLI data format converter with a built-in mapping language. It converts structured data between formats while giving users precise control over how fields are renamed, reshaped, filtered, and transformed — through a small, readable, purpose-built language.

The goal: **one tool to replace the dozen format-specific converters people cobble together with scripts.**

---

## Problem Statement

Developers, ops engineers, and data workers constantly convert between data formats:

- Config migration (YAML → TOML, JSON → YAML)
- Data pipeline glue (CSV → JSON, JSON Lines → CSV)
- API response reshaping (deeply nested JSON → flat CSV)
- Legacy format interop (XML → JSON, INI-like → TOML)

Current solutions fall into three buckets:

1. **One-off scripts** — Python/Ruby/Node snippets. Work, but fragile, slow to write, hard to share.
2. **jq / yq / xq family** — Powerful but format-specific, steep learning curves, inconsistent syntax across tools.
3. **Online converters** — Can't handle sensitive data, no automation, no customization.

None of them offer a **single tool** that reads multiple formats, writes multiple formats, and provides a **user-friendly transformation language** in between.

---

## Target Users

### Primary

- **Backend / DevOps engineers** — config file migration, CI/CD pipelines, infrastructure-as-code
- **Data engineers** — quick format conversions in ETL scripts, data cleaning
- **CLI power users** — people who live in the terminal and chain tools with pipes

### Secondary

- **Technical writers** — converting example data between formats for docs
- **Security researchers** — parsing/reshaping structured data from various tools
- **Students / learners** — understanding data format differences

---

## Core Features

### 1. Multi-Format I/O

Read and write all supported formats through a single unified interface.

**Supported formats (v1.0):**

| Format        | Read | Write | Priority | Notes |
|---------------|------|-------|----------|-------|
| JSON          | ✅   | ✅    | P0       | Streaming, pretty/compact output |
| YAML          | ✅   | ✅    | P0       | Multi-document, anchors/aliases |
| TOML          | ✅   | ✅    | P0       | Full spec compliance |
| CSV / TSV     | ✅   | ✅    | P0       | Headers, custom delimiters, quoting |
| XML           | ✅   | ✅    | P1       | Configurable attribute handling |
| JSON Lines    | ✅   | ✅    | P0       | Newline-delimited JSON |
| MessagePack   | ✅   | ✅    | P1       | Binary format |
| S-expressions | ✅   | ✅    | P2       | Lisp-style |
| Query String  | ✅   | ✅    | P2       | URL-encoded |
| EDN           | ✅   | ✅    | P2       | Clojure data notation |

**Format detection:**
- By file extension (`.json`, `.yaml`, `.yml`, `.toml`, `.csv`, `.xml`, etc.)
- By explicit flag (`-f json`, `-t yaml`)
- Stdin requires explicit format flag

### 2. Zero-Config Simple Conversion

The simplest use case should require zero learning:

```bash
morph -i input.json -o output.yaml
# or
cat data.csv | morph -f csv -t json
# or
morph -i data.json -t yaml  # outputs to stdout
```

No mapping file, no config, no flags beyond format specification.

### 3. The Mapping Language

A purpose-built DSL for describing data transformations. Design goals:

- **Readable at first sight** — someone unfamiliar should understand what a `.morph` file does
- **English-like keywords** — `rename`, `select`, `drop`, `where`, `set`, `cast`
- **Minimal syntax** — no brackets/parens unless necessary, arrow (`->`) for directionality
- **Composable** — operations apply in order, top to bottom
- **No Turing-completeness** — this is a data mapping language, not a programming language

#### 3.1 Core Operations

```
rename <path> -> <path>        # rename a field
select <path>, <path>, ...     # keep only these fields
drop <path>, <path>, ...       # remove these fields
flatten <path> -> <paths...>   # unnest object into flat fields
nest <paths...> -> <path>      # group flat fields into object
set <path> = <expression>      # create/overwrite field
default <path> = <expression>  # set only if field is null/missing
cast <path> as <type>          # type coercion (int, float, bool, string)
where <condition>              # filter rows/elements
sort <path> [asc|desc]         # sort elements
each <path> { ... }            # iterate over array elements
when <condition> { ... }       # conditional block
```

#### 3.2 Path Syntax

```
.field                   # top-level field
.parent.child            # nested field
.array[0]                # array index
.array[*]                # all array elements
.array[-1]               # last element
.["key with spaces"]     # quoted key
```

#### 3.3 Expressions

```
# Literals
"hello", 42, 3.14, true, false, null

# Field references
.field_name

# Function calls
join(.first, " ", .last)
lower(.name)
len(.items)

# Arithmetic
.price * .quantity
.total - .discount

# String interpolation
"{.first} {.last}"

# Comparison
.age > 18
.status == "active"
.name != null

# Logical
.age > 18 and .status == "active"
.role == "admin" or .role == "superadmin"
not .deleted
```

#### 3.4 Built-in Functions (v1.0)

**String:**
`join`, `split`, `lower`, `upper`, `trim`, `replace`, `starts_with`, `ends_with`, `contains`, `regex_match`, `regex_replace`, `substring`, `pad_left`, `pad_right`

**Math:**
`round`, `ceil`, `floor`, `abs`, `min`, `max`, `sum`

**Collection:**
`len`, `keys`, `values`, `flatten`, `unique`, `reverse`, `first`, `last`, `count`, `group_by`

**Type:**
`type_of`, `is_null`, `is_array`, `is_object`, `is_string`, `is_number`

**Date/Time:**
`now`, `parse_date`, `format_date`

**System:**
`env` (read environment variable)

**Utility:**
`coalesce` (first non-null), `if` (ternary)

### 4. CLI Interface

```
morph [OPTIONS]

INPUT/OUTPUT:
  -i, --input <FILE>        Input file (or stdin if omitted)
  -o, --output <FILE>       Output file (or stdout if omitted)
  -f, --from <FORMAT>       Input format (auto-detected from extension)
  -t, --to <FORMAT>         Output format (auto-detected from extension)

MAPPING:
  -m, --map <FILE>          Mapping file (.morph)
  -e, --expr <EXPRESSION>   Inline mapping expression
      --dry-run             Parse and validate mapping without executing

OUTPUT CONTROL:
      --pretty              Pretty-print output (default for terminal)
      --compact             Compact output (default for pipe)
      --indent <N>          Indentation width (default: 2)
      --no-color            Disable colored output
  -q, --quiet               Suppress warnings

FORMAT-SPECIFIC:
      --csv-delimiter <C>   CSV delimiter (default: ',')
      --csv-no-header       CSV has no header row
      --csv-header <FIELDS> Explicit CSV headers
      --xml-root <NAME>     XML root element name
      --xml-attr-prefix <S> XML attribute prefix (default: '@')
      --yaml-multi          Treat input as multi-document YAML

META:
  -V, --version             Print version
  -h, --help                Print help
      --formats             List all supported formats
      --functions           List all built-in functions
      --completions <SHELL> Generate shell completions
```

### 5. Error Handling

- **Clear error messages** with line numbers and context for mapping parse errors
- **Schema validation mode** — optionally validate output against a JSON Schema
- **Graceful degradation** — skip malformed records with `--skip-errors`, log to stderr
- **Type mismatch warnings** — warn when a cast might lose data

### 6. Performance

- **Streaming** for JSON, JSON Lines, CSV — don't buffer entire file in memory
- **Zero-copy parsing** where the format allows it
- **Parallel processing** for large files (optional, `--parallel`)
- Target: **>100MB/s** throughput for simple JSON→YAML conversion on modern hardware

---

## Architecture

```
┌─────────────┐     ┌──────────────┐     ┌──────────────┐     ┌─────────────┐
│   Reader     │────▶│  Universal   │────▶│  Transform   │────▶│   Writer    │
│  (format)    │     │  Value (UV)  │     │  Engine      │     │  (format)   │
└─────────────┘     └──────────────┘     └──────────────┘     └─────────────┘
                                               ▲
                                               │
                                         ┌─────┴─────┐
                                         │  Mapping   │
                                         │  Parser    │
                                         └───────────┘
```

### Universal Value (UV)

The internal representation all formats are normalized to:

```rust
enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    Map(IndexMap<String, Value>),  // preserves insertion order
}
```

Key design decisions:
- **Ordered maps** — preserves key order from input (important for config files)
- **Separate Int/Float** — avoids precision loss
- **Bytes variant** — supports binary formats like MessagePack
- **No format-specific metadata in UV** — XML attributes are mapped to a convention (e.g., `@attr`)

### Module Structure

```
morph/
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library root
│   ├── value.rs             # Universal Value type
│   ├── error.rs             # Error types
│   ├── formats/
│   │   ├── mod.rs           # Format registry, auto-detection
│   │   ├── json.rs          # JSON reader/writer
│   │   ├── yaml.rs          # YAML reader/writer
│   │   ├── toml.rs          # TOML reader/writer
│   │   ├── csv.rs           # CSV/TSV reader/writer
│   │   ├── xml.rs           # XML reader/writer
│   │   ├── jsonl.rs         # JSON Lines reader/writer
│   │   ├── msgpack.rs       # MessagePack reader/writer
│   │   ├── sexpr.rs         # S-expressions reader/writer
│   │   ├── querystring.rs   # Query string reader/writer
│   │   └── edn.rs           # EDN reader/writer
│   ├── mapping/
│   │   ├── mod.rs           # Mapping module root
│   │   ├── lexer.rs         # Tokenizer
│   │   ├── parser.rs        # AST construction
│   │   ├── ast.rs           # AST types
│   │   ├── eval.rs          # Mapping evaluator
│   │   └── functions.rs     # Built-in function implementations
│   └── cli.rs               # Argument parsing (clap)
├── tests/
│   ├── formats/             # Per-format round-trip tests
│   ├── mapping/             # Mapping language tests
│   └── integration/         # End-to-end CLI tests
├── Cargo.toml
├── README.md
├── PRD.md
├── LICENSE
└── .github/
    └── workflows/
        └── ci.yml           # Test + lint + build
```

---

## Milestones

### v0.1.0 — Foundation (MVP)
- [ ] Universal Value type
- [ ] JSON reader/writer
- [ ] YAML reader/writer
- [ ] TOML reader/writer
- [ ] CSV reader/writer
- [ ] CLI with `-i`, `-o`, `-f`, `-t` flags
- [ ] Format auto-detection by extension
- [ ] Pretty/compact output modes
- [ ] Basic error handling

### v0.2.0 — Mapping Language
- [ ] Lexer and parser
- [ ] Core operations: `rename`, `select`, `drop`, `set`, `default`, `cast`
- [ ] Path syntax (dot notation, array indexing)
- [ ] `-m` and `-e` flags
- [ ] String functions (`join`, `split`, `lower`, `upper`, `trim`, `replace`)
- [ ] Mapping validation (`--dry-run`)

### v0.3.0 — Advanced Mappings
- [ ] `flatten`, `nest` operations
- [ ] `where` filtering
- [ ] `sort` operation
- [ ] `each` and `when` blocks
- [ ] Arithmetic expressions
- [ ] String interpolation
- [ ] Collection functions (`keys`, `values`, `unique`, `group_by`)

### v0.4.0 — Extended Formats
- [ ] XML reader/writer
- [ ] JSON Lines reader/writer
- [ ] MessagePack reader/writer
- [ ] Format-specific options (CSV delimiters, XML attributes)
- [ ] Multi-document YAML

### v0.5.0 — Polish & Performance
- [ ] Streaming mode for large files
- [ ] Shell completions
- [ ] Comprehensive error messages with suggestions
- [ ] `--formats` and `--functions` help commands
- [ ] Performance benchmarks

### v1.0.0 — Stable Release
- [ ] S-expressions, Query String, EDN formats
- [ ] Schema validation
- [ ] `--skip-errors` mode
- [ ] Parallel processing
- [ ] Comprehensive documentation
- [ ] Homebrew / cargo-binstall distribution

---

## Non-Goals (v1.0)

- **GUI** — this is a CLI tool
- **Database connectivity** — not a database client
- **Network protocols** — no HTTP fetching built in (use curl | morph)
- **Turing-complete scripting** — the mapping language is intentionally limited
- **Format-specific validation** — morph converts, it doesn't validate schemas (except optionally)
- **Bidirectional sync** — one-way conversion only

---

## Competitive Landscape

| Tool       | Multi-format | Transform Language | Learning Curve | Streaming |
|------------|-------------|-------------------|----------------|-----------|
| jq         | JSON only   | jq DSL            | Steep          | ✅        |
| yq         | YAML/JSON   | jq-like           | Steep          | ✅        |
| xq         | XML→JSON    | jq-like           | Steep          | ❌        |
| csvkit     | CSV only    | SQL-like           | Medium         | Partial   |
| miller     | CSV/JSON/etc| Miller DSL         | Medium         | ✅        |
| dasel      | JSON/YAML/TOML/XML | Path queries | Low            | ❌        |
| **morph**  | **10+ formats** | **morph DSL** | **Low**        | **✅**    |

**Key differentiator:** morph combines the broadest format support with a **readable, learnable** transformation language. No need to learn jq's syntax — morph reads like English.

---

## Technical Decisions

### Why Rust?
- Single binary, no runtime dependencies
- Excellent performance for data processing
- Strong ecosystem for parsing (serde, nom)
- Cross-platform (Linux, macOS, Windows)
- Memory safety without GC (important for streaming)

### Why a custom DSL instead of embedding jq/Lua/etc?
- **Readability** — the #1 design goal. jq is powerful but cryptic for newcomers
- **Safety** — no arbitrary code execution, just data transformation
- **Learnability** — someone should understand a `.morph` file without reading docs
- **Optimization** — a purpose-built evaluator can optimize for common patterns

### Why not just wrap existing libraries?
- We do! Format readers/writers use battle-tested crates (`serde_json`, `serde_yaml`, `toml`, `quick-xml`, `csv`, `rmp-serde`)
- The value is in the **unified interface** and **mapping language**, not in reimplementing parsers

---

## Success Criteria

1. **Usability:** A developer can convert between any two supported formats in under 10 seconds
2. **Learnability:** The mapping language should be understood without reading docs for simple cases
3. **Performance:** Competitive with or faster than format-specific tools for simple conversions
4. **Adoption:** 500+ GitHub stars within 6 months of launch (indicates real-world utility)
5. **Reliability:** Zero data loss in format conversion for well-formed inputs
