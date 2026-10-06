# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `gud b` and `gud l` fetch from all remotes in the background, at startup and then
  every 30 seconds, refreshing the view when done. Fetches never prompt for credentials.
  Disable with `--no-fetch` or `git config gud.autoFetch false`; set the interval in
  seconds with `gud.fetchInterval` (`0` fetches only at startup).

## [0.1.5] - 2026-09-30

### Added

- `gud b`
  - `Tab` shows/hides remote branches.
  - `/` opens fuzzy search; `Esc` to cancel.

### Changed

- `gud b` can now delete the current branch.

## [0.1.4] - 2026-09-30

### Added

- `gud l`
  - `t` tags the selected commit; type the tag name and press `Enter` (`Esc` cancels).
  - `T` creates an annotated tag, opening the git editor for its message.

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

[Unreleased]: https://github.com/lkurcak/gud/compare/v0.1.5...HEAD
[0.1.5]: https://github.com/lkurcak/gud/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/lkurcak/gud/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/lkurcak/gud/compare/v0.1.2...v0.1.3
[0.1.1]: https://github.com/lkurcak/gud/releases/tag/v0.1.1
