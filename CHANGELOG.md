# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.8] - 2026-10-10

### Added

- Prebuilt binaries for Windows, macOS and Linux are attached to each
  [GitHub release](https://github.com/lkurcak/gud/releases).

## [0.1.7] - 2026-10-08

### Changed

- The tag name prompt in `gud log` now supports the same readline/emacs-style editing as the
  `gud branch` filter: move with arrows, `Ctrl-A`/`Ctrl-E`, `Ctrl-B`/`Ctrl-F`, `Alt-B`/`Alt-F`;
  delete with `Ctrl-D`, `Ctrl-W`, `Alt-D`, `Ctrl-K`, `Ctrl-U`; paste the last deletion with
  `Ctrl-Y`. The cursor is shown in place instead of always at the end.
- `Ctrl-Backspace` and `Ctrl-Delete` delete a word in text inputs.

## [0.1.6] - 2026-10-06

### Added

- `gud branch` and `gud log` fetch from all remotes in the background, at startup and then
  every 30 seconds, refreshing the view when done. Fetches never prompt for credentials.
  Disable with `--no-fetch` or `git config gud.autoFetch false`; set the interval in
  seconds with `gud.fetchInterval` (`0` fetches only at startup).

### Changed

- Commands now have long canonical names: `gud branch` and `gud log`.
  `b` and `l` remain as aliases, and `gud switch` is also an alias for `gud branch`.

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

[Unreleased]: https://github.com/lkurcak/gud/compare/v0.1.8...HEAD
[0.1.8]: https://github.com/lkurcak/gud/compare/v0.1.7...v0.1.8
[0.1.7]: https://github.com/lkurcak/gud/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/lkurcak/gud/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/lkurcak/gud/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/lkurcak/gud/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/lkurcak/gud/compare/v0.1.2...v0.1.3
[0.1.1]: https://github.com/lkurcak/gud/releases/tag/v0.1.1
