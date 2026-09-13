# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.0] - 2026-09-13

### Breaking

- `-n/--new` no longer overwrites an existing `.gitignore`. It is now an alias for the
  `new` subcommand and **merges** into the file instead. Anything relying on
  `git-ignore -n rust` regenerating a file from scratch needs `-f` now:
  `git-ignore -n rust -f`. The old spelling still succeeds rather than failing, so
  existing scripts change behaviour silently — worth checking any automation that uses it.

### Added

- `git-ignore new <preset>...` creates or extends `.gitignore` from gitignore.io presets.
  Presets are space- or comma-separated (`new go rust java`, `new go,rust`), deduplicated
  case-insensitively, and fetched in a single request.
  - Against an existing file, presets are merged: your file is kept and only patterns it
    doesn't already have are appended, under an `# Added by git-ignore:` heading. Template
    sections left with nothing new to add are dropped along with their comments, so
    re-running the same preset changes nothing.
  - `-f/--force` overwrites the existing file instead of merging.
  - An unknown preset fails before anything is written, naming the offending preset, including
    when it appears alongside valid ones (`new rust,notareallang`).
- `git-ignore verify` reports duplicate patterns and patterns that match nothing, exiting 1
  if it finds either, so it can be used in CI.
  - Duplicates ignore trailing whitespace, as git does. A repeat that follows an opposite
    (negated) pattern is not reported, since it still changes the result.
  - Match checking follows git's pattern semantics, covers everything under the current
    directory except `.git`, and reads no files outside it.
- `-V/--version` reports the version. The 1.0.0 binary had no version flag at all.

### Changed

- zsh completions are now a hand-written `_git-ignore` function rather than clap's generated
  script with string replacement applied to it. Preset names complete after `new` and
  `-n/--new`, subcommands and their options complete, and an empty cache prompts you to run
  `git-ignore update-cache`.

## [1.0.0] - 2026-02-04

### Added

- Initial release: append a pattern (`git-ignore <pattern>`), generate `.gitignore` from
  gitignore.io templates (`-n/--new`, comma-separated), list available templates
  (`-l/--list`), generate shell completions (`completions <shell>`), and cache template
  names for completion (`update-cache`).

[2.0.0]: https://github.com/uradical/git-ignore-cli/compare/v1.0.0...v2.0.0
[1.0.0]: https://github.com/uradical/git-ignore-cli/releases/tag/v1.0.0
