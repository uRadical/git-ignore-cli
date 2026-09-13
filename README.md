# git-ignore-cli

A CLI tool to manage `.gitignore` files. Quickly add patterns or generate complete templates from [gitignore.io](https://gitignore.io).

## Installation

```bash
cargo install --path .
```

Or build from source:

```bash
cargo build --release
```

The binary will be available at `target/release/git-ignore`.

## Usage

### Add a pattern to .gitignore

```bash
git-ignore node_modules
git-ignore "*.log"
git-ignore .env
```

Patterns are appended to the existing `.gitignore` file (or created if it doesn't exist). Duplicate patterns are detected and skipped.

### Create a .gitignore from presets

A preset is a language or tool name, such as `go`, `rust` or `java`:

```bash
git-ignore new rust
```

Combine as many as you like, space- or comma-separated:

```bash
git-ignore new go rust java
git-ignore new rust,visualstudiocode,macos
```

If there is no `.gitignore` yet, the preset is written as-is. If one already
exists, the presets are **merged into it**: your file is kept, and only patterns
it doesn't already have are appended under a `# Added by git-ignore` heading.
Template sections left with nothing new to add are skipped entirely, so a second
run of the same preset changes nothing.

To replace an existing `.gitignore` instead of merging, pass `-f`:

```bash
git-ignore new rust -f
```

> `-n/--new` is a deprecated alias for `new` and behaves identically, including
> the merge default: `git-ignore -n rust` merges, `git-ignore -n rust -f` overwrites.

### List available templates

```bash
git-ignore -l
```

### Verify a .gitignore

```bash
git-ignore verify
```

Checks the `.gitignore` in the current directory and reports:

- **Duplicate patterns** – a line that repeats an earlier one and has no effect. Repeats that follow an opposite (negated) pattern are not reported, since they still change the result.
- **Patterns that match nothing** – no file or directory under the current directory (excluding `.git`) matches the pattern.

Exits with status 1 if any problems are found, so it can be used in CI.

## Shell Completions

Generate completions for your shell:

```bash
# Bash
git-ignore completions bash >> ~/.bashrc

# Zsh
git-ignore completions zsh >> ~/.zshrc

# Fish
git-ignore completions fish > ~/.config/fish/completions/git-ignore.fish
```

For preset name completion after `new` (and the `-n` flag), first update the local cache:

```bash
git-ignore update-cache
```

Then regenerate your shell completions.

## Commands

| Command | Description |
|---------|-------------|
| `git-ignore <pattern>` | Add a pattern to `.gitignore` |
| `git-ignore new <preset>...` | Merge preset(s) into `.gitignore` |
| `git-ignore new <preset> -f` | Overwrite `.gitignore` with preset(s) |
| `git-ignore -l` | List available templates |
| `git-ignore completions <shell>` | Generate shell completions |
| `git-ignore update-cache` | Update local template cache |
| `git-ignore verify` | Check `.gitignore` for duplicate and unmatched patterns |

## License

MIT
