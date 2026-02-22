use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use indexmap::IndexMap;
use morph::cli::{parse_input, serialize_output, Format};
use morph::mapping::{eval::eval, lexer::tokenize, parser::parse};
use morph::value::Value;

// ---------------------------------------------------------------------------
// Data generators
// ---------------------------------------------------------------------------

/// Generate a JSON string representing an array of `n` objects.
fn gen_json_array(n: usize) -> String {
    let mut entries = Vec::with_capacity(n);
    for i in 0..n {
        entries.push(format!(
            r#"{{"id":{},"name":"user_{}","email":"user_{}@example.com","age":{},"active":{}}}"#,
            i,
            i,
            i,
            20 + (i % 50),
            if i % 3 == 0 { "true" } else { "false" }
        ));
    }
    format!("[{}]", entries.join(","))
}

/// Generate a CSV string with `n` rows (plus header).
fn gen_csv(n: usize) -> String {
    let mut out = String::from("id,name,email,age,active\n");
    for i in 0..n {
        out.push_str(&format!(
            "{},user_{},user_{}@example.com,{},{}\n",
            i,
            i,
            i,
            20 + (i % 50),
            i % 3 == 0
        ));
    }
    out
}

/// Generate a YAML string representing a list of `n` objects.
fn gen_yaml_array(n: usize) -> String {
    let mut out = String::new();
    for i in 0..n {
        out.push_str(&format!(
            "- id: {}\n  name: user_{}\n  email: user_{}@example.com\n  age: {}\n  active: {}\n",
            i,
            i,
            i,
            20 + (i % 50),
            i % 3 == 0
        ));
    }
    out
}

/// Build a morph Value representing a single object (for serialization benchmarks).
fn gen_value_object(i: usize) -> Value {
    let mut map = IndexMap::new();
    map.insert("id".to_string(), Value::Int(i as i64));
    map.insert("name".to_string(), Value::String(format!("user_{i}")));
    map.insert(
        "email".to_string(),
        Value::String(format!("user_{i}@example.com")),
    );
    map.insert("age".to_string(), Value::Int((20 + (i % 50)) as i64));
    map.insert("active".to_string(), Value::Bool(i.is_multiple_of(3)));
    Value::Map(map)
}

/// Build a morph Value array with `n` objects.
fn gen_value_array(n: usize) -> Value {
    Value::Array((0..n).map(gen_value_object).collect())
}

/// Parse a mapping expression string into a Program.
fn parse_mapping(input: &str) -> morph::mapping::ast::Program {
    let tokens = tokenize(input).unwrap();
    parse(tokens).unwrap()
}

// ---------------------------------------------------------------------------
// Benchmark groups
// ---------------------------------------------------------------------------

fn bench_json_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_json");
    for &count in &[100, 1_000, 10_000] {
        let input = gen_json_array(count);
        let size = input.len() as u64;
        group.throughput(Throughput::Bytes(size));
        group.bench_with_input(BenchmarkId::new("records", count), &input, |b, input| {
            b.iter(|| parse_input(black_box(input), Format::Json).unwrap());
        });
    }
    group.finish();
}

fn bench_csv_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_csv");
    for &count in &[100, 1_000, 10_000] {
        let input = gen_csv(count);
        let size = input.len() as u64;
        group.throughput(Throughput::Bytes(size));
        group.bench_with_input(BenchmarkId::new("rows", count), &input, |b, input| {
            b.iter(|| parse_input(black_box(input), Format::Csv).unwrap());
        });
    }
    group.finish();
}

fn bench_yaml_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_yaml");
    for &count in &[100, 1_000, 10_000] {
        let input = gen_yaml_array(count);
        let size = input.len() as u64;
        group.throughput(Throughput::Bytes(size));
        group.bench_with_input(BenchmarkId::new("records", count), &input, |b, input| {
            b.iter(|| parse_input(black_box(input), Format::Yaml).unwrap());
        });
    }
    group.finish();
}

fn bench_json_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialize_json");
    for &count in &[100, 1_000, 10_000] {
        let value = gen_value_array(count);
        group.bench_with_input(BenchmarkId::new("records", count), &value, |b, value| {
            b.iter(|| serialize_output(black_box(value), Format::Json, false).unwrap());
        });
    }
    group.finish();
}

fn bench_json_serialize_pretty(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialize_json_pretty");
    for &count in &[100, 1_000, 10_000] {
        let value = gen_value_array(count);
        group.bench_with_input(BenchmarkId::new("records", count), &value, |b, value| {
            b.iter(|| serialize_output(black_box(value), Format::Json, true).unwrap());
        });
    }
    group.finish();
}

fn bench_csv_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialize_csv");
    for &count in &[100, 1_000, 10_000] {
        let value = gen_value_array(count);
        group.bench_with_input(BenchmarkId::new("rows", count), &value, |b, value| {
            b.iter(|| serialize_output(black_box(value), Format::Csv, false).unwrap());
        });
    }
    group.finish();
}

fn bench_yaml_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialize_yaml");
    for &count in &[100, 1_000, 10_000] {
        let value = gen_value_array(count);
        group.bench_with_input(BenchmarkId::new("records", count), &value, |b, value| {
            b.iter(|| serialize_output(black_box(value), Format::Yaml, false).unwrap());
        });
    }
    group.finish();
}

fn bench_toml_serialize(c: &mut Criterion) {
    // TOML needs a top-level table; wrap the array under a key.
    let mut group = c.benchmark_group("serialize_toml");
    for &count in &[100, 1_000] {
        let mut map = IndexMap::new();
        map.insert("items".to_string(), gen_value_array(count));
        let value = Value::Map(map);
        group.bench_with_input(BenchmarkId::new("records", count), &value, |b, value| {
            b.iter(|| serialize_output(black_box(value), Format::Toml, false).unwrap());
        });
    }
    group.finish();
}

fn bench_json_to_yaml(c: &mut Criterion) {
    let mut group = c.benchmark_group("convert_json_to_yaml");
    for &count in &[100, 1_000, 10_000] {
        let input = gen_json_array(count);
        let size = input.len() as u64;
        group.throughput(Throughput::Bytes(size));
        group.bench_with_input(BenchmarkId::new("records", count), &input, |b, input| {
            b.iter(|| {
                let value = parse_input(black_box(input), Format::Json).unwrap();
                serialize_output(&value, Format::Yaml, false).unwrap()
            });
        });
    }
    group.finish();
}

fn bench_csv_to_json(c: &mut Criterion) {
    let mut group = c.benchmark_group("convert_csv_to_json");
    for &count in &[100, 1_000, 10_000] {
        let input = gen_csv(count);
        let size = input.len() as u64;
        group.throughput(Throughput::Bytes(size));
        group.bench_with_input(BenchmarkId::new("rows", count), &input, |b, input| {
            b.iter(|| {
                let value = parse_input(black_box(input), Format::Csv).unwrap();
                serialize_output(&value, Format::Json, false).unwrap()
            });
        });
    }
    group.finish();
}

fn bench_mapping_rename(c: &mut Criterion) {
    let mut group = c.benchmark_group("mapping_rename");
    let program = parse_mapping("rename .name -> .username");
    for &count in &[100, 1_000, 10_000] {
        let value = gen_value_array(count);
        group.bench_with_input(BenchmarkId::new("records", count), &value, |b, value| {
            b.iter(|| eval(black_box(&program), black_box(value)).unwrap());
        });
    }
    group.finish();
}

fn bench_mapping_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("mapping_filter");
    let program = parse_mapping("where .active == true");
    for &count in &[100, 1_000, 10_000] {
        let value = gen_value_array(count);
        group.bench_with_input(BenchmarkId::new("records", count), &value, |b, value| {
            b.iter(|| eval(black_box(&program), black_box(value)).unwrap());
        });
    }
    group.finish();
}

fn bench_mapping_complex(c: &mut Criterion) {
    let mut group = c.benchmark_group("mapping_complex");
    let program = parse_mapping(
        r#"rename .name -> .username
set .email_domain = "example.com"
drop .active
cast .age as string"#,
    );
    for &count in &[100, 1_000, 10_000] {
        let value = gen_value_array(count);
        group.bench_with_input(BenchmarkId::new("records", count), &value, |b, value| {
            b.iter(|| eval(black_box(&program), black_box(value)).unwrap());
        });
    }
    group.finish();
}

fn bench_end_to_end_json_json_with_mapping(c: &mut Criterion) {
    let mut group = c.benchmark_group("e2e_json_json_mapped");
    let program = parse_mapping("rename .name -> .username\nwhere .age > 30");
    for &count in &[100, 1_000, 10_000] {
        let input = gen_json_array(count);
        let size = input.len() as u64;
        group.throughput(Throughput::Bytes(size));
        group.bench_with_input(BenchmarkId::new("records", count), &input, |b, input| {
            b.iter(|| {
                let value = parse_input(black_box(input), Format::Json).unwrap();
                let mapped = eval(&program, &value).unwrap();
                serialize_output(&mapped, Format::Json, false).unwrap()
            });
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Group all benchmarks
// ---------------------------------------------------------------------------

criterion_group!(
    benches,
    // Parsing
    bench_json_parse,
    bench_csv_parse,
    bench_yaml_parse,
    // Serialization
    bench_json_serialize,
    bench_json_serialize_pretty,
    bench_csv_serialize,
    bench_yaml_serialize,
    bench_toml_serialize,
    // Conversions
    bench_json_to_yaml,
    bench_csv_to_json,
    // Mapping
    bench_mapping_rename,
    bench_mapping_filter,
    bench_mapping_complex,
    // End-to-end
    bench_end_to_end_json_json_with_mapping,
);
criterion_main!(benches);
