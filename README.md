# gud

[![crates.io](https://img.shields.io/crates/v/gud.svg)](https://crates.io/crates/gud)
[![Release](https://github.com/lkurcak/gud/actions/workflows/release.yml/badge.svg)](https://github.com/lkurcak/gud/actions/workflows/release.yml)
[![License](https://img.shields.io/crates/l/gud.svg)](#license)

Interactive git helper.

## Install

```sh
brew install lkurcak/tap/gud
```

or

```sh
cargo install gud
```

## Usage

```sh
gud b   # interactively switch to or delete local branches
gud l   # browse commits; tag, soft/hard reset or edit a commit message
```

While open, gud fetches from all remotes in the background: once at startup, then
every 5 minutes. Fetches never prompt for credentials; if a password or passphrase
would be needed, the fetch is silently skipped.

```sh
gud b --no-fetch                        # disable for one run
git config --global gud.autoFetch false # disable permanently
git config gud.fetchInterval 60         # seconds between fetches (0 = only at startup)
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
