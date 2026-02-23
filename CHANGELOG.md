# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-02-23

### Bug Fixes

- Resolve clippy warnings breaking CI on main ([#46](https://github.com/alvinreal/morph/pull/46)) ([8bd6718](https://github.com/alvinreal/morph/commit/8bd6718047478692ab80b69aed92513fc333a383))

### Documentation

- Replace summary with detailed cross-tool benchmark matrix ([9a47478](https://github.com/alvinreal/morph/commit/9a47478d024d6b4ffb502da5d0bde24f22a60ba6))
- Add at-a-glance performance comparison matrix ([76cd45a](https://github.com/alvinreal/morph/commit/76cd45ab8e50c15815755f3f65b951ad922d78aa))
- Add yq and miller head-to-head benchmark stats ([ca1f5dd](https://github.com/alvinreal/morph/commit/ca1f5ddd2bdc7ec8c60ce87e0e79c36efdc58e32))
- Replace placeholder perf claims with measured benchmark stats ([44c6f77](https://github.com/alvinreal/morph/commit/44c6f77a7ea87a7c5d7590dbd913c240dc167a0a))
- Refine blazing fast section with reproducible comparisons ([55ab5db](https://github.com/alvinreal/morph/commit/55ab5db168f5c040ed230dc97a06e63d0d2ffa4d))
- Add performance/benchmark section to README ([39a4b92](https://github.com/alvinreal/morph/commit/39a4b92fd1d4c31a7b0984f3c30ba52558619adc))
- Add comprehensive installation guide ([849d684](https://github.com/alvinreal/morph/commit/849d6842e3cade2902657fbf6e5b43011272b0ad))

### Features

- Add compatibility matrix CI with report artifacts ([bde03de](https://github.com/alvinreal/morph/commit/bde03de754b9fc9b9325bcaea9f1884b8ee1f029))
- Add cargo-binstall metadata for pre-built binary installs ([583e7c6](https://github.com/alvinreal/morph/commit/583e7c6eac225e328ff0adf166939eeb47f2a39c))
- Set up release-plz for automated versioning and changelog ([#74](https://github.com/alvinreal/morph/pull/74)) ([e6b26d5](https://github.com/alvinreal/morph/commit/e6b26d58feb0def7ef318fcbdcd314266dc2cdc4))
- Set up cargo-dist for automated builds and installers ([#72](https://github.com/alvinreal/morph/pull/72)) ([4dfcb51](https://github.com/alvinreal/morph/commit/4dfcb51774f8ee3b3f5d1d931deab8f70fdf33e9))
- Add Criterion benchmark suite for performance tracking ([#71](https://github.com/alvinreal/morph/pull/71)) ([783e280](https://github.com/alvinreal/morph/commit/783e2805d4a207816be065dc904ab9e8cc825e4a))
- Improve error UX with suggestions, context, and source snippets ([#70](https://github.com/alvinreal/morph/pull/70)) ([0c5f894](https://github.com/alvinreal/morph/commit/0c5f894d25759cc31ccf6d1274e3bbf9d2344bc6))
- Add shell completions and help commands ([#59](https://github.com/alvinreal/morph/pull/59)) ([60577e6](https://github.com/alvinreal/morph/commit/60577e6d55325762f4e1ea19ccf89c18ca892136))
- Add streaming mode for large files ([#58](https://github.com/alvinreal/morph/pull/58)) ([51411af](https://github.com/alvinreal/morph/commit/51411af9cad531ffe59dae098f20d1c8dd4fc371))
- Add format-specific CLI options ([#57](https://github.com/alvinreal/morph/pull/57)) ([ca37b8a](https://github.com/alvinreal/morph/commit/ca37b8ae525995f6ce605567fd72b0df18201e8f))
- Add MessagePack reader/writer ([#56](https://github.com/alvinreal/morph/pull/56)) ([3663e0d](https://github.com/alvinreal/morph/commit/3663e0d5950d55054f9f36caab2e91522ceb15fd))
- Add JSON Lines (JSONL/NDJSON) reader/writer ([#55](https://github.com/alvinreal/morph/pull/55)) ([46fe72c](https://github.com/alvinreal/morph/commit/46fe72cea9a72db4c77389fe2a83c71416ae7c26))
- Add XML format support (read/write) ([#54](https://github.com/alvinreal/morph/pull/54)) ([370932e](https://github.com/alvinreal/morph/commit/370932e6cacf23adc86452bbc0738cb77c680d54))
- Add collection functions, string interpolation, and if() ([#53](https://github.com/alvinreal/morph/pull/53)) ([13e7bd7](https://github.com/alvinreal/morph/commit/13e7bd7a518bd4f6e204266932d6af1c33d10a37))
- Implement each and when block statements ([#52](https://github.com/alvinreal/morph/pull/52)) ([3ab9ca7](https://github.com/alvinreal/morph/commit/3ab9ca7cee4672680392ca6db728e907e3f08e5c))
- Implement sort operation for arrays ([#51](https://github.com/alvinreal/morph/pull/51)) ([c5b0777](https://github.com/alvinreal/morph/commit/c5b0777c64b99a13ca6f216dbb3dc0237763dde2))
- Implement where filtering for arrays and values ([#50](https://github.com/alvinreal/morph/pull/50)) ([ad53de5](https://github.com/alvinreal/morph/commit/ad53de576d8fff45f8ba0c008b7e15192a841662))
- Implement flatten and nest mapping operations ([#49](https://github.com/alvinreal/morph/pull/49)) ([cb3e230](https://github.com/alvinreal/morph/commit/cb3e230c036b2636f6d1fe1d9fd5cef4b49694d7))
- Add -m, -e, and --dry-run CLI mapping flags ([#48](https://github.com/alvinreal/morph/pull/48)) ([2a8100e](https://github.com/alvinreal/morph/commit/2a8100e9a8616041da4c74619787feaad39d70b0))
- Add comprehensive tests for built-in string functions ([#47](https://github.com/alvinreal/morph/pull/47)) ([a05a0ef](https://github.com/alvinreal/morph/commit/a05a0efad548f54c31bfe3e289f14010c6ed0be6))
- Implement mapping evaluator with built-in functions ([#45](https://github.com/alvinreal/morph/pull/45)) ([5e489fc](https://github.com/alvinreal/morph/commit/5e489fc796b081ccbf471b5ef5a965c654bdd5c0))
- Implement mapping language parser with AST and expression support ([#44](https://github.com/alvinreal/morph/pull/44)) ([b7a3524](https://github.com/alvinreal/morph/commit/b7a3524e2497198c14fbea658b0b8bdf99f7d55b))
- Implement mapping language lexer with full token set ([#43](https://github.com/alvinreal/morph/pull/43)) ([fb4fe38](https://github.com/alvinreal/morph/commit/fb4fe38fded398ce1e76bdc2c51f7344980d9ce5))
- Add cross-format integration tests and CLI end-to-end tests ([#42](https://github.com/alvinreal/morph/pull/42)) ([10b944d](https://github.com/alvinreal/morph/commit/10b944d6f129c3d00be2597f5edbd85363cd26bc))
- Expand CSV reader/writer with config, delimiters, and tests ([#41](https://github.com/alvinreal/morph/pull/41)) ([a005106](https://github.com/alvinreal/morph/commit/a005106b804f6c79d022a0313cda5187ea9a5cbd))
- Expand TOML reader/writer with comprehensive tests ([#40](https://github.com/alvinreal/morph/pull/40)) ([0fd4421](https://github.com/alvinreal/morph/commit/0fd442179813ffef1652869e00419341fa7bbbf8))
- Expand YAML test coverage with multi-doc, anchors, edge cases ([#39](https://github.com/alvinreal/morph/pull/39)) ([81daa10](https://github.com/alvinreal/morph/commit/81daa106687b0afc32ac281398cb79f7f90edaaf))
- Comprehensive JSON reader/writer with full test coverage ([#38](https://github.com/alvinreal/morph/pull/38)) ([ed6c8ab](https://github.com/alvinreal/morph/commit/ed6c8abc6a95016ddc0ddf58ded8c5f9e6569bd3))
- Implement CLI argument parsing and main conversion pipeline ([#37](https://github.com/alvinreal/morph/pull/37)) ([8af9b57](https://github.com/alvinreal/morph/commit/8af9b5795fe2f4eddcc66f02dfa10af04a7275f0))
- Implement error type hierarchy ([2f25a00](https://github.com/alvinreal/morph/commit/2f25a00e55e3b0ecd97fbfcf8657eb288bb82f6b))
- Project scaffold with Cargo.toml and module stubs ([#33](https://github.com/alvinreal/morph/pull/33)) ([5da5874](https://github.com/alvinreal/morph/commit/5da58744cbc33389d419804df80691e8cf410d20))

### Performance

- Add competitive benchmark comparisons and blazing fast positioning ([398ba65](https://github.com/alvinreal/morph/commit/398ba65de463d94ad3e05848dee8150d67804255))

### Styling

- Fix formatting in error test ([640b309](https://github.com/alvinreal/morph/commit/640b30995b8057fb298a35488af78797de5acfec))

### Features

- Universal data format converter with mapping language
- Support for JSON, YAML, TOML, CSV, XML, MessagePack, JSON Lines formats
- Mapping language with rename, select, drop, set, cast, where, each, when, flatten, nest, sort, default operations
- 30+ built-in functions (string, math, collection, type operations)
- Streaming mode for large files (JSONL, CSV, JSON array)
- Shell completions for bash, zsh, fish, PowerShell, elvish
- Error UX with "did you mean?" suggestions for formats and functions
- Performance benchmark suite with Criterion
- cargo-dist setup for automated release builds
