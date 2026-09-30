# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.5] - 2026-09-30

### Added

- `gud b` can list and check out remote-only branches. Press `Tab` to show
  them; selecting one creates a local tracking branch.
- `/` in `gud b` opens fuzzy search over all branches.

### Changed

- `gud b` can now delete the currently checked-out branch.

## [0.1.4] - 2026-09-30

### Added

- `t` in `gud l` tags the selected commit; type the tag name and press `Enter`
  (`Esc` cancels). `T` creates an annotated tag, opening the git editor for
  its message.

## [0.1.3] - 2026-09-29

### Added

- `gud l` command to interactively browse commits, with a preview pane
  (message and diffstat) on terminals at least 80 columns wide.
  - Navigate with arrow keys, `j`/`k`, `g`/`G`, `PgUp`/`PgDn`; more history
    loads as you scroll.
  - `Enter` shows the full commit in git's pager.
  - `e` edits the selected commit's message in the git editor, rewriting any
    commits above it. Only messages change, so it never conflicts, keeps
    merges intact and leaves the index and working tree alone.
  - `r` soft-resets to the selected commit; `R` hard-resets after confirmation.

## [0.1.1] - 2026-09-25

### Added

- `gud b` command to interactively switch to or delete local branches.
  - Navigate with arrow keys, `j`/`k`, `g`/`G` (or `Home`/`End`).
  - `Enter` switches to the selected branch.
  - `d` deletes the selected branch; `D` force-deletes it.
  - `q`, `Esc` or `Ctrl-C` quits.
- Release workflow publishing to crates.io on `v*` tags.
- Dual licensing under MIT or Apache-2.0.

[Unreleased]: https://github.com/lkurcak/gud/compare/v0.1.5...HEAD
[0.1.5]: https://github.com/lkurcak/gud/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/lkurcak/gud/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/lkurcak/gud/compare/v0.1.2...v0.1.3
[0.1.1]: https://github.com/lkurcak/gud/releases/tag/v0.1.1
