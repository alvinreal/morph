# Migration Guide: jq / yq / mlr → morph

Moving from `jq`, `yq`, or `mlr` (Miller) to morph? This guide shows side-by-side recipes for common tasks so you can translate your existing scripts.

> **Key difference:** morph uses a purpose-built mapping language instead of a general expression language. It's more readable for data transformations but intentionally less powerful than a full programming language.

---

## Table of Contents

- [Format Conversion](#format-conversion)
- [Field Selection](#field-selection)
- [Field Renaming](#field-renaming)
- [Filtering Rows](#filtering-rows)
- [Adding / Computing Fields](#adding--computing-fields)
- [Dropping Fields](#dropping-fields)
- [Type Casting](#type-casting)
- [Nested Data](#nested-data)
- [Sorting](#sorting)
- [String Operations](#string-operations)
- [Conditional Logic](#conditional-logic)
- [Working with Arrays](#working-with-arrays)
- [Defaults / Null Handling](#defaults--null-handling)
- [Chaining Operations](#chaining-operations)
- [Caveats & Differences](#caveats--differences)

---

## Format Conversion

**Convert JSON → YAML**

```bash
# jq (needs yq or manual pipeline)
cat data.json | yq -y .

# yq
yq -o yaml data.json

# morph
morph -i data.json -o data.yaml
```

**Convert CSV → JSON**

```bash
# mlr
mlr --icsv --ojson cat data.csv

# jq (no native CSV support — needs external tool)
# Not directly possible

# morph
morph -i data.csv -o data.json
```

**Convert YAML → TOML**

```bash
# yq
yq -o toml data.yaml

# morph
morph -i data.yaml -o data.toml
```

**Pipe-friendly (stdin/stdout)**

```bash
# jq
echo '{"a":1}' | jq .

# morph
echo '{"a":1}' | morph -f json -t yaml
```

---

## Field Selection

**Select specific fields from objects**

```bash
# jq
jq '{name, email}' data.json
# or from array:
jq '[.[] | {name, email}]' data.json

# yq
yq '{.name, .email}' data.yaml

# mlr
mlr --json cut -f name,email data.json

# morph (inline)
morph -i data.json -o out.json -e 'select .name, .email'
```

```morph
# morph mapping file
select .name, .email
```

---

## Field Renaming

**Rename a field**

```bash
# jq
jq '.new_name = .old_name | del(.old_name)' data.json
# or for arrays:
jq '[.[] | .new_name = .old_name | del(.old_name)]' data.json

# yq
yq '.new_name = .old_name | del(.old_name)' data.yaml

# mlr
mlr --json rename old_name,new_name data.json

# morph
morph -i data.json -o out.json -e 'rename .old_name -> .new_name'
```

```morph
# morph mapping file
rename .firstName -> .first_name
rename .lastName  -> .last_name
```

---

## Filtering Rows

**Filter array elements by condition**

```bash
# jq
jq '[.[] | select(.age > 18)]' data.json

# yq
yq '[.[] | select(.age > 18)]' data.yaml

# mlr
mlr --json filter '$age > 18' data.json

# morph
morph -i data.json -o out.json -e 'where .age > 18'
```

**Multiple conditions**

```bash
# jq
jq '[.[] | select(.age > 18 and .active == true)]' data.json

# mlr
mlr --json filter '$age > 18 && $active == "true"' data.json

# morph
morph -i data.json -o out.json -e 'where .age > 18 && .active == true'
```

---

## Adding / Computing Fields

**Add a new field**

```bash
# jq
jq '.role = "user"' data.json
# for arrays:
jq '[.[] | .role = "user"]' data.json

# yq
yq '.role = "user"' data.yaml

# mlr
mlr --json put '$role = "user"' data.json

# morph
morph -i data.json -o out.json -e 'set .role = "user"'
```

**Computed field from existing data**

```bash
# jq
jq '.full_name = (.first + " " + .last)' data.json

# mlr
mlr --json put '$full_name = $first . " " . $last' data.json

# morph
morph -i data.json -o out.json -e 'set .full_name = join(.first, " ", .last)'
```

---

## Dropping Fields

**Remove fields**

```bash
# jq
jq 'del(.password, .internal_id)' data.json
# for arrays:
jq '[.[] | del(.password, .internal_id)]' data.json

# yq
yq 'del(.password, .internal_id)' data.yaml

# mlr
mlr --json cut -x -f password,internal_id data.json

# morph
morph -i data.json -o out.json -e 'drop .password, .internal_id'
```

---

## Type Casting

**Convert string to integer**

```bash
# jq
jq '.age = (.age | tonumber)' data.json

# mlr
mlr --json put '$age = int($age)' data.json

# morph
morph -i data.json -o out.json -e 'cast .age as int'
```

**morph cast types:** `int`, `float`, `string`, `bool`

```morph
cast .age as int
cast .price as float
cast .active as bool
cast .count as string
```

---

## Nested Data

**Flatten nested object**

```bash
# jq
jq '{address_street: .address.street, address_city: .address.city} + del(.address)' data.json

# mlr
mlr --json nest --explode-values --across-fields -f address data.json

# morph
morph -i data.json -o out.json -e 'flatten .address'
```

**Nest flat fields into object**

```bash
# jq
jq '{address: {street: .address_street, city: .address_city}} + del(.address_street, .address_city)' data.json

# morph
morph -i data.json -o out.json -e 'nest .address_street, .address_city -> .address'
```

---

## Sorting

**Sort array by field**

```bash
# jq
jq 'sort_by(.name)' data.json

# mlr
mlr --json sort-by name data.json

# morph
morph -i data.json -o out.json -e 'sort .name'
```

**Sort descending**

```bash
# jq
jq 'sort_by(.age) | reverse' data.json

# mlr
mlr --json sort-by age -nr data.json

# morph
morph -i data.json -o out.json -e 'sort .age desc'
```

---

## String Operations

**Lowercase / Uppercase**

```bash
# jq
jq '.name |= ascii_downcase' data.json

# mlr
mlr --json put '$name = strmatch($name, ".*")' data.json

# morph
morph -i data.json -o out.json -e 'set .name = lower(.name)'
```

**String replacement**

```bash
# jq
jq '.title |= gsub(" "; "-")' data.json

# morph
morph -i data.json -o out.json -e 'set .slug = replace(.title, " ", "-")'
```

**Available morph string functions:** `join()`, `split()`, `lower()`, `upper()`, `trim()`, `replace()`, `len()`

---

## Conditional Logic

**Apply operations conditionally**

```bash
# jq
jq 'if .type == "admin" then .permissions = ["all"] else . end' data.json

# mlr
mlr --json put 'if ($type == "admin") { $permissions = "all" }' data.json

# morph
morph -i data.json -o out.json -e 'when .type == "admin" { set .permissions = "all" }'
```

```morph
# morph mapping file
when .type == "admin" {
  set .permissions = "all"
  set .elevated = true
}
```

---

## Working with Arrays

**Transform each element**

```bash
# jq
jq '.items |= [.[] | .price = (.price * 1.1)]' data.json

# morph
morph -i data.json -o out.json -m transform.morph
```

```morph
# transform.morph
each .items {
  rename .product_name -> .name
  cast .quantity as int
}
```

---

## Defaults / Null Handling

**Set default values for missing fields**

```bash
# jq
jq '.role //= "user"' data.json
# or for arrays:
jq '[.[] | .role //= "user"]' data.json

# mlr
mlr --json put 'if (is_not_present($role)) { $role = "user" }' data.json

# morph
morph -i data.json -o out.json -e 'default .role = "user"'
```

**morph also supports `coalesce()`:**

```morph
set .display_name = coalesce(.nickname, .full_name, .email)
```

---

## Chaining Operations

**Multiple transformations in sequence**

```bash
# jq (piped expressions)
jq '[.[] | select(.active) | {name, email} | .name |= ascii_downcase]' data.json

# mlr (verb chaining)
mlr --json filter '$active == "true"' then cut -f name,email data.json

# morph (mapping file — operations apply top to bottom)
morph -i data.json -o out.json -m pipeline.morph
```

```morph
# pipeline.morph
where .active == true
select .name, .email
set .name = lower(.name)
```

**morph inline chaining** (semicolons or newlines):

```bash
morph -i data.json -o out.json -e 'where .active == true
select .name, .email
set .name = lower(.name)'
```

---

## Caveats & Differences

### What morph does differently

| Aspect | jq / yq | mlr | morph |
|--------|---------|-----|-------|
| **Paradigm** | Expression language | Verb-based DSL | Statement-based DSL |
| **Array handling** | Manual `[.[] \| ...]` | Implicit per-record | Automatic for most operations |
| **Format support** | JSON (jq) / YAML (yq) | CSV, JSON, others | JSON, YAML, TOML, CSV, XML, MsgPack, JSONL |
| **Learning curve** | Steep (functional) | Moderate | Low (English-like) |
| **Turing complete** | Yes | Yes | No (by design) |
| **Recursion** | Yes (`.. \| ...`) | No | No |
| **Raw text processing** | `@text`, `@csv`, etc. | Yes | No — structured data only |

### Things morph can't do (yet)

- **Recursive descent** — jq's `..` operator to search all levels. morph requires explicit paths.
- **Arbitrary computation** — jq can do math, regex, string interpolation, user-defined functions. morph focuses on structural transformations.
- **Multi-file joins** — mlr's `join` verb. morph processes one input at a time.
- **In-place editing** — jq's `sponge` pattern or yq's `-i` flag. morph always writes to a separate output.
- **Custom functions** — jq's `def`. morph has built-in functions only.

### Common gotchas when migrating

1. **Array vs single object:** morph's `where`, `sort`, and similar operations automatically work on arrays. You don't need the `[.[] | ...]` wrapper from jq.

2. **Path syntax:** morph uses `.field` (dot prefix), similar to jq. But nested paths are `.a.b.c`, not `.a | .b | .c`.

3. **No assignment chaining:** In jq you can do `.a = .b | .c = .d`. In morph, each operation is a separate statement.

4. **String quoting:** morph uses double quotes `"..."` for string literals. Single quotes are not supported in mapping expressions.

5. **Boolean values:** morph uses `true` / `false` (not `"true"` / `"false"` strings). Use `cast .field as bool` if your data has string booleans.

---

## Quick Reference Card

| Task | morph |
|------|-------|
| Convert format | `morph -i in.json -o out.yaml` |
| Select fields | `select .name, .email` |
| Rename field | `rename .old -> .new` |
| Filter rows | `where .age > 18` |
| Add field | `set .role = "user"` |
| Remove field | `drop .password` |
| Set default | `default .role = "user"` |
| Cast type | `cast .age as int` |
| Flatten | `flatten .address` |
| Nest | `nest .a, .b -> .group` |
| Sort | `sort .name` |
| Transform array items | `each .items { ... }` |
| Conditional | `when .x == "y" { ... }` |
| Lowercase | `set .name = lower(.name)` |
| Concatenate | `set .full = join(.first, " ", .last)` |

---

📖 **[Full mapping language reference](MAPPING_LANGUAGE.md)** | 📦 **[Installation guide](INSTALLATION.md)**
