use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
use ignore::{WalkBuilder, gitignore::GitignoreBuilder};
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const GITIGNORE_API: &str = "https://www.toptal.com/developers/gitignore/api";

#[derive(Parser)]
#[command(name = "git-ignore")]
#[command(version)]
#[command(about = "Manage your .gitignore file", long_about = None)]
struct Cli {
    /// File, directory, or pattern to add to .gitignore
    pattern: Option<String>,

    /// Deprecated alias for the `new` subcommand
    /// Supports comma-separated values: rust,visualstudiocode
    #[arg(short = 'n', long = "new", value_name = "PRESET")]
    new: Option<String>,

    /// Overwrite an existing .gitignore instead of merging into it
    #[arg(short = 'f', long = "force")]
    force: bool,

    /// List available templates
    #[arg(short = 'l', long = "list")]
    list: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Create or extend .gitignore from preset(s), e.g. `new go rust`
    New {
        /// Preset name(s): space- or comma-separated, e.g. `go rust` or `go,rust`
        #[arg(required = true, value_name = "PRESET")]
        presets: Vec<String>,

        /// Overwrite an existing .gitignore instead of merging into it
        #[arg(short = 'f', long = "force")]
        force: bool,
    },
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Update the local template cache (for shell completions)
    UpdateCache,
    /// Check .gitignore for duplicate patterns and patterns that match nothing
    Verify,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(command) = cli.command {
        match command {
            Commands::New { presets, force } => {
                new_gitignore(&presets, force)?;
            }
            Commands::Completions { shell } => {
                generate_completions(shell)?;
            }
            Commands::UpdateCache => {
                update_cache()?;
            }
            Commands::Verify => {
                if !verify_gitignore()? {
                    std::process::exit(1);
                }
            }
        }
        return Ok(());
    }

    if cli.list {
        list_templates()?;
    } else if let Some(preset) = cli.new {
        new_gitignore(&[preset], cli.force)?;
    } else if let Some(pattern) = cli.pattern {
        add_pattern(&pattern)?;
    } else {
        println!("Usage: git-ignore <pattern>       Add pattern to .gitignore");
        println!("       git-ignore new <preset>    Merge preset patterns into .gitignore");
        println!("       git-ignore new go,rust     Combine multiple presets");
        println!("       git-ignore new rust -f     Overwrite .gitignore with the preset");
        println!("       git-ignore -l              List available templates");
        println!("       git-ignore completions     Generate shell completions");
        println!("       git-ignore update-cache    Update template cache");
        println!("       git-ignore verify          Check for duplicate or unmatched patterns");
    }

    Ok(())
}

fn get_cache_dir() -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir()
        .context("Could not find cache directory")?
        .join("git-ignore");
    fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir)
}

fn get_cache_path() -> Result<PathBuf> {
    Ok(get_cache_dir()?.join("templates.txt"))
}

fn update_cache() -> Result<()> {
    let url = format!("{}/list", GITIGNORE_API);

    println!("Fetching template list...");
    let response = reqwest::blocking::get(&url).context("Failed to fetch template list")?;

    let content = response.text()?;

    let templates: Vec<&str> = content
        .split([',', '\n'])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let cache_path = get_cache_path()?;
    fs::write(&cache_path, templates.join("\n"))?;

    println!(
        "Cached {} templates to {}",
        templates.len(),
        cache_path.display()
    );
    Ok(())
}

fn read_cached_templates() -> Result<Vec<String>> {
    let cache_path = get_cache_path()?;
    if cache_path.exists() {
        let content = fs::read_to_string(&cache_path)?;
        Ok(content.lines().map(|s| s.to_string()).collect())
    } else {
        Ok(vec![])
    }
}

fn generate_completions(shell: Shell) -> Result<()> {
    let mut cmd = Cli::command();

    match shell {
        Shell::Bash => {
            generate(shell, &mut cmd, "git-ignore", &mut io::stdout());
            // Add custom completion for -n flag using cached templates
            let templates = read_cached_templates().unwrap_or_default();
            if !templates.is_empty() {
                println!();
                println!("# Custom completion for -n/--new flag");
                println!("_git_ignore_templates() {{");
                println!("    local templates=\"{}\"", templates.join(" "));
                println!(
                    "    COMPREPLY=($(compgen -W \"$templates\" -- \"${{COMP_WORDS[COMP_CWORD]}}\"))"
                );
                println!("}}");
                println!("complete -F _git_ignore_templates git-ignore -n");
                println!("complete -F _git_ignore_templates git-ignore --new");
            }
        }
        Shell::Zsh => {
            let templates = read_cached_templates().unwrap_or_default();
            let template_list = if templates.is_empty() {
                String::new()
            } else {
                templates.join(" ")
            };
            print!("{}", zsh_completion(&template_list));
        }
        Shell::Fish => {
            generate(shell, &mut cmd, "git-ignore", &mut io::stdout());
            let templates = read_cached_templates().unwrap_or_default();
            if !templates.is_empty() {
                println!();
                println!("# Custom completion for -n/--new flag");
                for tmpl in &templates {
                    println!("complete -c git-ignore -s n -l new -xa '{}'", tmpl);
                }
            }
        }
        _ => {
            generate(shell, &mut cmd, "git-ignore", &mut io::stdout());
        }
    }

    eprintln!();
    eprintln!("# Run 'git-ignore update-cache' first to enable template completion");

    Ok(())
}

fn zsh_completion(templates: &str) -> String {
    format!(
        r#"#compdef git-ignore

_git-ignore() {{
  local curcontext="$curcontext" state

  # Handle subcommands
  if (( CURRENT > 2 )) && [[ "$words[2]" != -* ]]; then
    case $words[2] in
      completions)
        if (( CURRENT == 3 )); then
          local -a shells
          shells=(bash elvish fish powershell zsh)
          _describe 'shell' shells
        fi
        return ;;
      new)
        if [[ "$PREFIX" == -* ]]; then
          local -a opts
          opts=('-f:Overwrite an existing .gitignore' '--force:Overwrite an existing .gitignore')
          _describe 'option' opts
        else
          __git_ignore_templates
        fi
        return ;;
      update-cache|verify|help) return ;;
    esac
  fi

  # Complete -n/--new value
  if [[ "$words[CURRENT-1]" == -n || "$words[CURRENT-1]" == --new ]]; then
    __git_ignore_templates
    return
  fi

  # Complete options and subcommands
  if [[ "$PREFIX" == -* ]]; then
    local -a opts
    opts=(
      '-n:Generate a new .gitignore for the specified template(s)'
      '--new:Generate a new .gitignore for the specified template(s)'
      '-l:List available templates'
      '--list:List available templates'
      '-h:Print help'
      '--help:Print help'
    )
    _describe 'option' opts
  else
    local -a commands
    commands=(
      'new:Create or extend .gitignore from preset(s)'
      'completions:Generate shell completions'
      'update-cache:Update the local template cache'
      'verify:Check .gitignore for duplicate and unmatched patterns'
      'help:Print this message or the help of the given subcommand(s)'
    )
    _describe 'command' commands
  fi
}}

__git_ignore_templates() {{
  local -a templates
  templates=({templates})
  if (( $#templates )); then
    _describe 'template' templates
  else
    _message 'run "git-ignore update-cache" first'
  fi
}}

_git-ignore "$@"
"#,
        templates = templates
    )
}

fn add_pattern(pattern: &str) -> Result<()> {
    let gitignore_path = std::path::Path::new(".gitignore");

    // Check if pattern already exists
    if gitignore_path.exists() {
        let contents = fs::read_to_string(gitignore_path)?;
        let lines: Vec<&str> = contents.lines().collect();

        if lines.iter().any(|line| line.trim() == pattern.trim()) {
            println!("Pattern '{}' already exists in .gitignore", pattern);
            return Ok(());
        }
    }

    // Append pattern to .gitignore
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(gitignore_path)
        .context("Failed to open .gitignore")?;

    // Add newline before pattern if file doesn't end with one
    if gitignore_path.exists() {
        let contents = fs::read_to_string(gitignore_path)?;
        if !contents.is_empty() && !contents.ends_with('\n') {
            writeln!(file)?;
        }
    }

    writeln!(file, "{}", pattern)?;
    println!("Added '{}' to .gitignore", pattern);

    Ok(())
}

/// Creates .gitignore from the given presets, merging into an existing file
/// unless `force` is set, in which case the file is replaced.
fn new_gitignore(presets: &[String], force: bool) -> Result<()> {
    let presets = parse_presets(presets);
    if presets.is_empty() {
        anyhow::bail!("No preset given. Example: git-ignore new rust,go");
    }
    let label = presets.join(", ");
    let path = Path::new(".gitignore");

    let existing = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => String::new(),
        Err(err) => return Err(err).context("Failed to read .gitignore"),
    };

    println!("Fetching .gitignore template for '{}'...", label);
    let template = fetch_template(&presets)?;

    // Nothing worth keeping in an absent or blank file, so just write the template
    if force || existing.trim().is_empty() {
        fs::write(path, &template).context("Failed to write .gitignore")?;
        if existing.trim().is_empty() {
            println!("Generated .gitignore for {}", label);
        } else {
            println!("Overwrote .gitignore with {}", label);
        }
        return Ok(());
    }

    let (block, added) = merge_addition(&existing, &template, &presets);
    if added == 0 {
        println!(".gitignore already has every pattern from {}", label);
        return Ok(());
    }

    let mut contents = existing;
    if !contents.ends_with('\n') {
        contents.push('\n');
    }
    contents.push('\n');
    contents.push_str(&block);
    fs::write(path, contents).context("Failed to write .gitignore")?;

    println!(
        "Merged {} new pattern(s) from {} into .gitignore",
        added, label
    );
    println!("Run 'git-ignore verify' to check the result");

    Ok(())
}

/// Splits comma-separated presets out of each argument, dropping repeats.
fn parse_presets(args: &[String]) -> Vec<String> {
    let mut presets: Vec<String> = Vec::new();
    for arg in args {
        for preset in arg.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            if !presets.iter().any(|seen| seen.eq_ignore_ascii_case(preset)) {
                presets.push(preset.to_string());
            }
        }
    }
    presets
}

fn fetch_template(presets: &[String]) -> Result<String> {
    let url = format!("{}/{}", GITIGNORE_API, presets.join(","));
    let response = reqwest::blocking::get(&url).context("Failed to fetch template")?;
    let status = response.status();
    let content = response.text()?;

    if !status.is_success() {
        // The API names the offending preset in a `#!! ERROR: ... !!#` line
        if let Some(error) = content.lines().find(|line| line.contains("ERROR:")) {
            anyhow::bail!(
                "{}",
                error.trim_matches(|c| c == '#' || c == '!' || c == ' ')
            );
        }
        anyhow::bail!(
            "Preset '{}' not found. Use 'git-ignore -l' to list available presets.",
            presets.join(",")
        );
    }

    Ok(content)
}

/// Builds the block to append to `existing`, keeping only template patterns it
/// doesn't already have, and how many patterns that block adds. Template
/// sections left with no new pattern are dropped, comments and all.
fn merge_addition<'a>(existing: &'a str, template: &'a str, presets: &[String]) -> (String, usize) {
    let mut seen: HashSet<&str> = parse_rules(existing)
        .iter()
        .map(|rule| rule.pattern)
        .collect();
    let mut sections: Vec<String> = Vec::new();
    let mut section: Vec<&str> = Vec::new();
    let mut kept_pattern = false;
    let mut added = 0;

    // The trailing empty line flushes the final section
    for line in template.lines().chain(std::iter::once("")) {
        let line = normalize(line);
        if line.is_empty() {
            if kept_pattern {
                sections.push(section.join("\n"));
            }
            section.clear();
            kept_pattern = false;
        } else if line.starts_with('#') {
            section.push(line);
        } else if seen.insert(line) {
            section.push(line);
            kept_pattern = true;
            added += 1;
        }
    }

    if added == 0 {
        return (String::new(), 0);
    }

    let block = format!(
        "# Added by git-ignore: {}\n{}\n",
        presets.join(", "),
        sections.join("\n\n")
    );
    (block, added)
}

fn list_templates() -> Result<()> {
    let url = format!("{}/list", GITIGNORE_API);

    let response = reqwest::blocking::get(&url).context("Failed to fetch template list")?;

    let content = response.text()?;

    // The API returns comma-separated values in lines
    let templates: Vec<&str> = content
        .split([',', '\n'])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    println!("Available templates ({}):", templates.len());
    for template in templates {
        println!("  {}", template);
    }

    Ok(())
}

/// A non-blank, non-comment line of a .gitignore file.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Rule<'a> {
    line: usize,
    pattern: &'a str,
}

impl Rule<'_> {
    fn is_negated(&self) -> bool {
        self.pattern.starts_with('!')
    }
}

/// Reports problems in ./.gitignore. Returns false if any were found.
fn verify_gitignore() -> Result<bool> {
    let contents = fs::read_to_string(".gitignore").context("Failed to read .gitignore")?;
    let root = std::env::current_dir()?;

    let rules = parse_rules(&contents);
    let duplicates = find_duplicates(&rules);
    let unique: Vec<Rule> = rules
        .iter()
        .copied()
        .filter(|rule| !duplicates.iter().any(|(dup, _)| dup.line == rule.line))
        .collect();
    let unmatched = find_unmatched(&root, &unique)?;

    if duplicates.is_empty() && unmatched.is_empty() {
        println!("No problems found ({} patterns checked)", rules.len());
        return Ok(true);
    }

    if !duplicates.is_empty() {
        println!("Duplicate patterns:");
        for (dup, earlier) in &duplicates {
            println!(
                "  line {}: {} (same as line {})",
                dup.line, dup.pattern, earlier.line
            );
        }
        println!();
    }

    if !unmatched.is_empty() {
        println!("Patterns that match nothing:");
        for rule in &unmatched {
            println!("  line {}: {}", rule.line, rule.pattern);
        }
        println!();
    }

    println!(
        "{} duplicate(s), {} unmatched pattern(s)",
        duplicates.len(),
        unmatched.len()
    );
    Ok(false)
}

/// Trailing spaces are ignored by git unless escaped with a backslash.
fn normalize(line: &str) -> &str {
    if line.ends_with("\\ ") {
        line
    } else {
        line.trim_end()
    }
}

fn parse_rules(contents: &str) -> Vec<Rule<'_>> {
    contents
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let pattern = normalize(line);
            (!pattern.is_empty() && !pattern.starts_with('#')).then_some(Rule {
                line: i + 1,
                pattern,
            })
        })
        .collect()
}

/// Finds patterns that repeat an earlier line, as (duplicate, earlier) pairs.
/// A repeat that follows an opposite pattern (e.g. `*.log`, `!keep.log`, `*.log`)
/// still changes the outcome for some paths, so it is not reported.
fn find_duplicates<'a>(rules: &[Rule<'a>]) -> Vec<(Rule<'a>, Rule<'a>)> {
    let mut duplicates = Vec::new();
    for (i, rule) in rules.iter().enumerate() {
        let Some(j) = rules[..i].iter().rposition(|r| r.pattern == rule.pattern) else {
            continue;
        };
        if rules[j + 1..i]
            .iter()
            .all(|r| r.is_negated() == rule.is_negated())
        {
            duplicates.push((*rule, rules[j]));
        }
    }
    duplicates
}

/// Finds patterns that match no file or directory under `root` (ignoring `.git`).
fn find_unmatched<'a>(root: &Path, rules: &[Rule<'a>]) -> Result<Vec<Rule<'a>>> {
    let mut pending = Vec::with_capacity(rules.len());
    for rule in rules {
        let mut builder = GitignoreBuilder::new(root);
        builder
            .add_line(None, rule.pattern)
            .with_context(|| format!("Invalid pattern on line {}: {}", rule.line, rule.pattern))?;
        pending.push((*rule, builder.build()?));
    }

    let walker = WalkBuilder::new(root)
        .standard_filters(false)
        .filter_entry(|entry| entry.file_name() != ".git")
        .build();

    for entry in walker {
        if pending.is_empty() {
            break;
        }
        let entry = entry?;
        if entry.depth() == 0 {
            continue;
        }
        let is_dir = entry.file_type().is_some_and(|t| t.is_dir());
        pending.retain(|(_, matcher)| matcher.matched(entry.path(), is_dir).is_none());
    }

    Ok(pending.into_iter().map(|(rule, _)| rule).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rules_skips_comments_and_blank_lines() {
        let rules = parse_rules("# comment\n\ntarget/  \n\\#literal\nfoo\\ \n");
        let parsed: Vec<(usize, &str)> = rules.iter().map(|r| (r.line, r.pattern)).collect();
        assert_eq!(parsed, [(3, "target/"), (4, "\\#literal"), (5, "foo\\ ")]);
    }

    #[test]
    fn duplicates_reference_nearest_earlier_line() {
        let rules = parse_rules("target/\n*.log\ntarget/\n*.log\ntarget/\n");
        let pairs: Vec<(usize, usize)> = find_duplicates(&rules)
            .iter()
            .map(|(dup, earlier)| (dup.line, earlier.line))
            .collect();
        assert_eq!(pairs, [(3, 1), (4, 2), (5, 3)]);
    }

    #[test]
    fn duplicates_after_opposite_pattern_are_not_reported() {
        let rules = parse_rules("*.log\n!keep.log\n*.log\n!a\nb\n!a\n");
        assert!(find_duplicates(&rules).is_empty());
    }

    #[test]
    fn presets_split_on_commas_and_drop_repeats() {
        let args = [
            "go,rust".to_string(),
            " java ".to_string(),
            "Go".to_string(),
        ];
        assert_eq!(parse_presets(&args), ["go", "rust", "java"]);
    }

    #[test]
    fn merge_skips_present_patterns_and_emptied_sections() {
        let existing = "# mine\ntarget/\n";
        let template = concat!(
            "# Created by gitignore.io\n\n",
            "### Rust ###\n# Generated by Cargo\ndebug/\ntarget/\n\n",
            "### Already covered ###\ntarget/\n\n",
            "# End of gitignore.io\n"
        );

        let (block, added) = merge_addition(existing, template, &["rust".to_string()]);

        assert_eq!(added, 1);
        assert_eq!(
            block,
            "# Added by git-ignore: rust\n### Rust ###\n# Generated by Cargo\ndebug/\n"
        );
    }

    #[test]
    fn merge_adds_nothing_when_every_pattern_is_present() {
        let (block, added) = merge_addition(
            "debug/\ntarget/\n",
            "### Rust ###\ndebug/\ntarget/\n",
            &["rust".to_string()],
        );
        assert!(block.is_empty());
        assert_eq!(added, 0);
    }

    #[test]
    fn merge_keeps_only_the_first_copy_of_a_repeated_template_pattern() {
        let template = "### Go ###\n*.exe\n\n### Rust ###\n*.exe\n*.pdb\n";
        let (block, added) = merge_addition("", template, &["go".to_string()]);

        assert_eq!(added, 2);
        assert_eq!(
            block,
            "# Added by git-ignore: go\n### Go ###\n*.exe\n\n### Rust ###\n*.pdb\n"
        );
    }

    #[test]
    fn unmatched_patterns_are_reported() -> Result<()> {
        let root = std::env::temp_dir().join(format!("git-ignore-verify-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src"))?;
        fs::create_dir_all(root.join("target/debug"))?;
        fs::create_dir_all(root.join(".git"))?;
        fs::write(root.join("src/main.rs"), "")?;
        fs::write(root.join("target/debug/app"), "")?;
        fs::write(root.join(".git/HEAD"), "")?;

        let rules =
            parse_rules("target/\ndebug/\n*.rs\n!src/main.rs\n/main.rs\napp/\n*.pdb\nHEAD\n");
        let unmatched = find_unmatched(&root, &rules);
        fs::remove_dir_all(&root)?;

        let unmatched: Vec<&str> = unmatched?.iter().map(|r| r.pattern).collect();
        assert_eq!(unmatched, ["/main.rs", "app/", "*.pdb", "HEAD"]);
        Ok(())
    }
}
