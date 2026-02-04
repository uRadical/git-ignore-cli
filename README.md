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

### Generate a .gitignore from templates

Generate a `.gitignore` for a specific language or tool:

```bash
git-ignore -n rust
git-ignore -n python
git-ignore -n node
```

Combine multiple templates:

```bash
git-ignore -n rust,visualstudiocode,macos
```

### List available templates

```bash
git-ignore -l
```

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

For template name completion with the `-n` flag, first update the local cache:

```bash
git-ignore update-cache
```

Then regenerate your shell completions.

## Commands

| Command | Description |
|---------|-------------|
| `git-ignore <pattern>` | Add a pattern to `.gitignore` |
| `git-ignore -n <template>` | Generate `.gitignore` from template(s) |
| `git-ignore -l` | List available templates |
| `git-ignore completions <shell>` | Generate shell completions |
| `git-ignore update-cache` | Update local template cache |

## License

MIT
