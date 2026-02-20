# Mapping Language Specification

## Overview

The morph mapping language is a domain-specific language for describing data transformations between formats. It prioritizes **readability** — a `.morph` file should be understandable at first glance.

## Design Philosophy

1. **English-like** — keywords are common English verbs
2. **Top-to-bottom** — operations apply in order of appearance
3. **Explicit** — the arrow `->` shows direction, paths show location
4. **Safe** — no side effects, no I/O, no arbitrary code

## Syntax Reference

### Comments

```morph
# This is a comment
# Comments start with # and extend to end of line
```

### Paths

Paths reference locations in the data structure:

```morph
.name                     # top-level field "name"
.user.email               # nested: user → email
.users[0]                 # first element of users array
.users[-1]                # last element
.users[*]                 # all elements (wildcard)
.users[*].name            # name field of every user
.["field with spaces"]    # quoted key for special characters
```

### Operations

#### rename — Rename a field

```morph
rename .old_name -> .new_name
rename .user.firstName -> .user.first_name
```

#### select — Keep only specified fields

```morph
select .name, .email, .role
select .user.name, .user.email    # works with nested paths
```

All unselected fields are discarded.

#### drop — Remove specific fields

```morph
drop .password, .internal_id, ._metadata
```

#### set — Create or overwrite a field

```morph
set .full_name = join(.first_name, " ", .last_name)
set .created_at = now()
set .status = "active"
set .score = .points * 10
```

#### default — Set a field only if null or missing

```morph
default .role = "user"
default .created_at = now()
default .tags = []
```

#### cast — Type coercion

```morph
cast .age as int
cast .price as float
cast .active as bool
cast .count as string
```

Supported types: `int`, `float`, `bool`, `string`

#### flatten — Unnest an object into flat fields

```morph
# { address: { street: "...", city: "..." } }
# becomes: { address_street: "...", address_city: "..." }
flatten .address

# With custom prefix
flatten .address -> prefix "addr"
# becomes: { addr_street: "...", addr_city: "..." }
```

#### nest — Group flat fields into an object

```morph
# { address_street: "...", address_city: "..." }
# becomes: { address: { street: "...", city: "..." } }
nest .address_street, .address_city, .address_zip -> .address
```

#### where — Filter elements

```morph
where .age >= 18
where .status == "active"
where .name != null
where .score > 50 and .verified == true
```

#### sort — Sort array elements

```morph
sort .name asc
sort .created_at desc
sort .score desc, .name asc    # multi-key sort
```

#### each — Iterate over array elements

```morph
each .items {
  rename .product_name -> .name
  cast .quantity as int
  set .total = .price * .quantity
}
```

#### when — Conditional transformation

```morph
when .type == "admin" {
  set .permissions = ["read", "write", "delete"]
  set .dashboard = true
}

when .age < 13 {
  drop .email
  set .restricted = true
}
```

### Expressions

#### Literals

```morph
"hello"       # string
42            # integer
3.14          # float
true / false  # boolean
null          # null
[]            # empty array
["a", "b"]   # array literal
```

#### Arithmetic

```morph
.price * .quantity
.total - .discount
.count + 1
.value / 100
.amount % 10          # modulo
```

#### Comparison

```morph
.age > 18
.status == "active"
.name != null
.score >= 50
.count <= 100
```

#### Logical

```morph
.age > 18 and .verified == true
.role == "admin" or .role == "super"
not .deleted
```

#### String Interpolation

```morph
set .greeting = "Hello, {.name}!"
set .url = "https://api.example.com/users/{.id}"
```

### Functions

#### String Functions

| Function | Description | Example |
|----------|-------------|---------|
| `join(a, b, ...)` | Concatenate values | `join(.first, " ", .last)` |
| `split(s, delim)` | Split string | `split(.tags, ",")` |
| `lower(s)` | Lowercase | `lower(.name)` |
| `upper(s)` | Uppercase | `upper(.code)` |
| `trim(s)` | Strip whitespace | `trim(.input)` |
| `replace(s, old, new)` | Replace substring | `replace(.name, " ", "_")` |
| `starts_with(s, prefix)` | Check prefix | `starts_with(.url, "https")` |
| `ends_with(s, suffix)` | Check suffix | `ends_with(.file, ".json")` |
| `contains(s, sub)` | Check contains | `contains(.text, "error")` |
| `substring(s, start, len)` | Extract substring | `substring(.id, 0, 8)` |
| `pad_left(s, len, char)` | Left-pad | `pad_left(.num, 5, "0")` |
| `pad_right(s, len, char)` | Right-pad | `pad_right(.name, 20, " ")` |
| `regex_match(s, pattern)` | Regex test | `regex_match(.email, ".*@.*")` |
| `regex_replace(s, pat, rep)` | Regex replace | `regex_replace(.text, "\\d+", "#")` |

#### Math Functions

| Function | Description | Example |
|----------|-------------|---------|
| `round(n)` | Round to nearest int | `round(.score)` |
| `ceil(n)` | Round up | `ceil(.price)` |
| `floor(n)` | Round down | `floor(.price)` |
| `abs(n)` | Absolute value | `abs(.delta)` |
| `min(a, b)` | Minimum | `min(.x, .y)` |
| `max(a, b)` | Maximum | `max(.x, .y)` |
| `sum(arr)` | Sum of array | `sum(.scores)` |

#### Collection Functions

| Function | Description | Example |
|----------|-------------|---------|
| `len(x)` | Length | `len(.items)` |
| `keys(obj)` | Object keys | `keys(.config)` |
| `values(obj)` | Object values | `values(.config)` |
| `unique(arr)` | Deduplicate | `unique(.tags)` |
| `reverse(arr)` | Reverse order | `reverse(.items)` |
| `first(arr)` | First element | `first(.results)` |
| `last(arr)` | Last element | `last(.results)` |
| `count(arr, cond)` | Count matching | `count(.items, .active)` |
| `group_by(arr, key)` | Group elements | `group_by(.users, .role)` |
| `flatten(arr)` | Flatten nested arrays | `flatten(.nested)` |

#### Type Functions

| Function | Description | Example |
|----------|-------------|---------|
| `type_of(x)` | Get type name | `type_of(.value)` |
| `is_null(x)` | Check null | `is_null(.field)` |
| `is_array(x)` | Check array | `is_array(.data)` |
| `is_object(x)` | Check object | `is_object(.config)` |
| `is_string(x)` | Check string | `is_string(.name)` |
| `is_number(x)` | Check number | `is_number(.age)` |

#### Utility Functions

| Function | Description | Example |
|----------|-------------|---------|
| `coalesce(a, b, ...)` | First non-null | `coalesce(.nickname, .name, "anon")` |
| `if(cond, then, else)` | Ternary | `if(.age >= 18, "adult", "minor")` |
| `now()` | Current ISO timestamp | `set .timestamp = now()` |
| `env(name)` | Environment variable | `set .api_key = env("API_KEY")` |
| `parse_date(s, fmt)` | Parse date string | `parse_date(.date, "%Y-%m-%d")` |
| `format_date(d, fmt)` | Format date | `format_date(.date, "%d/%m/%Y")` |

## Examples

### Flatten a REST API response for CSV export

```morph
# Input: array of user objects from API
# Output: flat CSV

each . {
  rename .id -> .user_id
  set .full_name = join(.first_name, " ", .last_name)
  set .city = .address.city
  set .country = .address.country
  drop .address, .first_name, .last_name
  cast .created_at as string
}

select .user_id, .full_name, .email, .city, .country, .role, .created_at
sort .user_id asc
```

### Migrate YAML config to TOML

```morph
# Rename keys from kebab-case to snake_case
rename .database-host -> .database_host
rename .database-port -> .database_port
rename .max-connections -> .max_connections

# Set defaults for new TOML-specific fields
default .database_host = "localhost"
cast .database_port as int
cast .max_connections as int
cast .debug as bool
```

### Clean CSV data

```morph
# Normalize messy CSV
each . {
  set .name = trim(.name)
  set .email = lower(trim(.email))
  cast .age as int
  default .country = "US"
  
  when .phone == "" {
    set .phone = null
  }
}

where .email != null
where .age > 0
sort .name asc
```
