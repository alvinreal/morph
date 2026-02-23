# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
