# Release Candidate Process

This document defines the release candidate (RC) checklist for morph releases, including documentation review, migration notes, and rollback procedures.

---

## Table of Contents

- [Overview](#overview)
- [RC Lifecycle](#rc-lifecycle)
- [RC1 Checklist](#rc1-checklist)
- [RC2 Checklist](#rc2-checklist)
- [Sign-Off Criteria](#sign-off-criteria)
- [Release Procedure](#release-procedure)
- [Rollback Plan](#rollback-plan)
- [Post-Release](#post-release)

---

## Overview

morph follows [Semantic Versioning](https://semver.org/). Each minor release goes through at least one Release Candidate (RC) cycle before the final release. The RC process ensures quality, compatibility, and documentation are production-ready.

### Versioning Scheme

- **RC tags:** `v0.X.0-rc.1`, `v0.X.0-rc.2`, …
- **Final release:** `v0.X.0`
- RCs are published as GitHub pre-releases (not promoted to `latest`).

---

## RC Lifecycle

```
main ──► RC1 tag ──► testing/feedback ──► RC2 tag (if needed) ──► final tag
              │                                │
              └── fix on main, cherry-pick ─────┘
```

1. **Feature freeze** — all planned features merged to `main`.
2. **RC1** — first candidate tagged; testing begins.
3. **Feedback window** — minimum 3 days for RC1, 2 days for subsequent RCs.
4. **RC2+** — only if blocking issues found in previous RC.
5. **Final release** — when an RC passes all sign-off criteria.

---

## RC1 Checklist

### Code Quality

- [ ] All CI checks pass on `main` (stable + nightly, all OS targets)
- [ ] No open issues labeled `blocker` or `critical`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` clean
- [ ] `cargo fmt --all -- --check` clean
- [ ] `cargo test` — all tests pass
- [ ] Golden conformance suite passes (`tests/golden_conformance.rs`)

### Documentation

- [ ] `README.md` — feature table, install instructions, and examples are current
- [ ] `docs/MAPPING_LANGUAGE.md` — all operations and functions documented
- [ ] `docs/INSTALLATION.md` — all install methods verified
- [ ] `docs/MIGRATION.md` — jq/yq/mlr recipes updated for any new features
- [ ] `CHANGELOG.md` — unreleased section reviewed and accurate
- [ ] `CONTRIBUTING.md` — project structure and guidelines current
- [ ] Inline help (`morph --help`, `morph --formats`, `morph --functions`) matches docs

### Migration & Compatibility

- [ ] Breaking changes documented in `CHANGELOG.md` with migration steps
- [ ] `docs/MIGRATION.md` covers any new operations or changed behavior
- [ ] Compatibility matrix CI passes (all OS × toolchain combinations)
- [ ] Minimum Supported Rust Version (MSRV) documented if changed

### Performance

- [ ] Benchmark suite runs without errors (`cargo bench`)
- [ ] No significant regressions vs. previous release snapshot
- [ ] `bench-results/` snapshot generated and committed
- [ ] README performance table updated if numbers changed materially

### Release Infrastructure

- [ ] `Cargo.toml` version bumped to target version (e.g., `0.6.0`)
- [ ] `dist-workspace.toml` configuration reviewed
- [ ] `release-plz.toml` configuration reviewed
- [ ] `cliff.toml` changelog configuration current
- [ ] GitHub Actions workflows (`ci.yml`, `bench.yml`, `release.yml`) current

### Tag & Publish RC1

```bash
# Ensure main is clean and up to date
git checkout main && git pull

# Tag the RC
git tag -a v0.X.0-rc.1 -m "v0.X.0 Release Candidate 1"
git push origin v0.X.0-rc.1
```

The `release.yml` workflow will build artifacts and create a GitHub pre-release.

---

## RC2 Checklist

RC2 is only needed if blocking issues were found in RC1.

### Fixes

- [ ] All RC1 feedback issues resolved
- [ ] Fixes merged to `main` with tests
- [ ] No new features — only bug fixes and doc corrections

### Verification

- [ ] Full CI passes on `main`
- [ ] Golden conformance suite passes
- [ ] Benchmark suite shows no new regressions
- [ ] Fixed issues verified manually

### Documentation

- [ ] `CHANGELOG.md` updated with RC1→RC2 fixes
- [ ] Any doc corrections from RC1 feedback applied

### Tag & Publish RC2

```bash
git tag -a v0.X.0-rc.2 -m "v0.X.0 Release Candidate 2"
git push origin v0.X.0-rc.2
```

---

## Sign-Off Criteria

A release candidate is approved for final release when **all** of the following are met:

| Criterion | Description |
|-----------|-------------|
| **CI Green** | All matrix cells pass (3 OS × 2 toolchains) |
| **Zero Blockers** | No open issues labeled `blocker` or `critical` |
| **Conformance** | Golden conformance suite 100% pass |
| **Performance** | No regression >10% vs. previous release |
| **Docs Complete** | All user-facing docs reviewed and accurate |
| **Feedback Window** | Minimum soak time elapsed (3 days RC1, 2 days RC2+) |
| **Breaking Changes** | All breaking changes documented with migration path |
| **Changelog** | `CHANGELOG.md` reviewed and finalized |

---

## Release Procedure

Once an RC is signed off:

### 1. Finalize Changelog

```bash
# Generate changelog from commits
git cliff --tag v0.X.0 -o CHANGELOG.md

# Review and edit if needed
$EDITOR CHANGELOG.md

git add CHANGELOG.md
git commit -m "docs: finalize changelog for v0.X.0"
git push origin main
```

### 2. Tag Final Release

```bash
git tag -a v0.X.0 -m "v0.X.0"
git push origin v0.X.0
```

### 3. Verify Release Artifacts

The `release.yml` workflow will:
- Build binaries for all target platforms
- Create a GitHub Release with artifacts
- Publish the Homebrew formula to `alvinreal/homebrew-tap`
- `release-plz` will handle crates.io publishing

Verify:
- [ ] GitHub Release page has all platform binaries
- [ ] Install script works: `curl ... | sh`
- [ ] `cargo binstall morph` works (after crates.io publish)
- [ ] `brew install alvinreal/tap/morph` works (after Homebrew publish)

### 4. Announce

- [ ] GitHub Release notes are clear and complete
- [ ] Update any external links or documentation sites

---

## Rollback Plan

If a critical issue is discovered after a final release:

### Severity Assessment

| Severity | Action | Timeline |
|----------|--------|----------|
| **Critical** (data loss, crashes on valid input) | Yank + patch release | Immediate |
| **High** (incorrect output, security issue) | Patch release | Within 24h |
| **Medium** (edge case bugs, doc errors) | Next minor release | Normal cycle |
| **Low** (cosmetic, minor UX) | Next minor release | Normal cycle |

### Yank Procedure (Critical Only)

```bash
# 1. Yank the crates.io release
cargo yank --version 0.X.0

# 2. Mark GitHub Release as pre-release (hides from "latest")
gh release edit v0.X.0 --prerelease

# 3. Add warning to release notes
gh release edit v0.X.0 --notes "⚠️ This release has been yanked due to [issue]. Please use v0.X.1 instead."
```

### Patch Release

```bash
# 1. Create a release branch from the tag
git checkout -b release/v0.X.1 v0.X.0

# 2. Cherry-pick or apply the fix
git cherry-pick <fix-commit>

# 3. Bump version in Cargo.toml to 0.X.1
# 4. Update CHANGELOG.md
# 5. Tag and push
git tag -a v0.X.1 -m "v0.X.1 — hotfix for [issue]"
git push origin v0.X.1

# 6. Merge fix back to main
git checkout main
git merge release/v0.X.1
git push origin main
```

### Homebrew Rollback

If the Homebrew formula needs to revert:

```bash
# In the homebrew-tap repo
git revert HEAD  # revert the formula update
git push
```

### Communication

For critical/high severity rollbacks:
1. Update GitHub Release notes with warning
2. Open a GitHub Issue documenting the problem and fix
3. Reference the fix in the patch release notes

---

## Post-Release

After a successful release:

- [ ] Verify all install methods work with the new version
- [ ] Archive the benchmark snapshot for this release
- [ ] Update the compatibility matrix results in the repo
- [ ] Bump `Cargo.toml` version to next dev version (e.g., `0.X+1.0`)
- [ ] Create the next milestone's epic issue if planned
- [ ] Close the current milestone/epic

---

## Quick Reference

| Step | Command |
|------|---------|
| Run tests | `cargo test` |
| Run clippy | `cargo clippy --all-targets --all-features -- -D warnings` |
| Check formatting | `cargo fmt --all -- --check` |
| Run benchmarks | `cargo bench` |
| Generate changelog | `git cliff --tag v0.X.0 -o CHANGELOG.md` |
| Tag RC | `git tag -a v0.X.0-rc.1 -m "v0.X.0 RC1"` |
| Tag release | `git tag -a v0.X.0 -m "v0.X.0"` |
| Yank crate | `cargo yank --version 0.X.0` |
