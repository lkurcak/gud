# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-09-25

### Added

- `gud b` command to interactively switch to or delete local branches.
  - Navigate with arrow keys, `j`/`k`, `g`/`G` (or `Home`/`End`).
  - `Enter` switches to the selected branch.
  - `d` deletes the selected branch; `D` force-deletes it.
  - `q`, `Esc` or `Ctrl-C` quits.
- Release workflow publishing to crates.io on `v*` tags.
- Dual licensing under MIT or Apache-2.0.

[Unreleased]: https://github.com/lkurcak/gud/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/lkurcak/gud/releases/tag/v0.1.1
