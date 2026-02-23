# Benchmark Results

This directory holds versioned benchmark snapshots produced by the CI
[Benchmarks workflow](../.github/workflows/bench.yml) and the local
[`scripts/bench-report.sh`](../scripts/bench-report.sh) script.

## How It Works

### CI (Automated)

Every push to `main` and every pull request triggers the **Benchmarks**
workflow. It:

1. Runs the full Criterion benchmark suite (`cargo bench --bench benchmarks`).
2. Parses the bencher-format output into a structured **JSON snapshot**
   containing version, commit, date, Rust toolchain, and per-benchmark
   timings.
3. Generates a human-readable **Markdown report** posted to the GitHub
   Actions step summary.
4. Uploads both artifacts (`benchmark-results`) with 90-day retention.
5. On PRs, adds a comparison note so reviewers can check for regressions
   against the latest `main` run.

### Local (Manual)

Run the helper script from the repo root:

```bash
./scripts/bench-report.sh              # outputs to bench-results/
./scripts/bench-report.sh /tmp/bench   # custom output directory
```

This produces the same JSON + text snapshot pair and a `bench-latest.*`
copy for quick access.

## Snapshot Format

Each snapshot is a JSON file named
`bench-snapshot-<version>-<YYYY-MM-DD>.json`:

```json
{
  "version": "0.1.0",
  "date": "2026-02-23",
  "timestamp": "2026-02-23T22:38:00Z",
  "commit": "abc1234...",
  "commit_short": "abc1234",
  "rust_version": "rustc 1.82.0 ...",
  "os": "Linux 6.5.0-...",
  "arch": "x86_64",
  "benchmarks": [
    {
      "name": "parse_json/records/1000",
      "ns_per_iter": 123456,
      "deviation": 789
    }
  ]
}
```

## Using Snapshots for Release Notes

Compare the `bench-latest.json` from the current release with the
previous release's snapshot to quantify performance changes:

```bash
# Quick diff of two snapshots
python3 scripts/bench-compare.py \
  bench-results/bench-snapshot-0.1.0-2026-01-01.json \
  bench-results/bench-snapshot-0.2.0-2026-02-23.json
```

Or manually compare the `ns_per_iter` values in the JSON files.

## Tracked Benchmarks

The suite covers the full pipeline:

| Group | What it measures |
|-------|-----------------|
| `parse_*` | Deserialization speed (JSON, CSV, YAML) |
| `serialize_*` | Serialization speed (JSON, CSV, YAML, TOML) |
| `convert_*` | End-to-end format conversion |
| `mapping_*` | Mapping language operations (rename, filter, complex) |
| `e2e_*` | Full pipeline: parse → map → serialize |

Each group tests at 100 / 1,000 / 10,000 record sizes to show scaling
behaviour.
