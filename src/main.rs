use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

const GITIGNORE_API: &str = "https://www.toptal.com/developers/gitignore/api";

#[derive(Parser)]
#[command(name = "git-ignore")]
#[command(about = "Manage your .gitignore file", long_about = None)]
struct Cli {
    /// File, directory, or pattern to add to .gitignore
    pattern: Option<String>,

    /// Generate a new .gitignore for the specified language/template(s)
    /// Supports comma-separated values: rust,visualstudiocode
    #[arg(short = 'n', long = "new")]
    new: Option<String>,

    /// List available templates
    #[arg(short = 'l', long = "list")]
    list: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Update the local template cache (for shell completions)
    UpdateCache,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(command) = cli.command {
        match command {
            Commands::Completions { shell } => {
                generate_completions(shell)?;
            }
            Commands::UpdateCache => {
                update_cache()?;
            }
        }
        return Ok(());
    }

    if cli.list {
        list_templates()?;
    } else if let Some(lang) = cli.new {
        generate_gitignore(&lang)?;
    } else if let Some(pattern) = cli.pattern {
        add_pattern(&pattern)?;
    } else {
        println!("Usage: git-ignore <pattern>       Add pattern to .gitignore");
        println!("       git-ignore -n <lang>       Generate .gitignore for language");
        println!("       git-ignore -n a,b,c        Combine multiple templates");
        println!("       git-ignore -l              List available templates");
        println!("       git-ignore completions     Generate shell completions");
        println!("       git-ignore update-cache    Update template cache");
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

            // Generate completions to a buffer so we can modify them
            let mut buf = Vec::new();
            generate(shell, &mut cmd, "git-ignore", &mut buf);
            let completions = String::from_utf8_lossy(&buf);

            if !templates.is_empty() {
                // Replace _default with _git_ignore_new for -n/--new flags
                let modified = completions.replace("]:NEW:_default'", "]:NEW:_git_ignore_new'");
                print!("{}", modified);

                // Add the custom completion function
                println!();
                println!("# Custom completion for -n/--new flag");
                println!("_git_ignore_new() {{");
                println!("    local -a templates");
                println!("    templates=({})", templates.join(" "));
                println!("    _describe 'template' templates");
                println!("}}");
            } else {
                print!("{}", completions);
            }
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
    eprintln!("# Note: Completions only work with 'git-ignore' (hyphen), not 'git ignore' (space)");

    Ok(())
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

fn generate_gitignore(lang: &str) -> Result<()> {
    let url = format!("{}/{}", GITIGNORE_API, lang);

    println!("Fetching .gitignore template for '{}'...", lang);

    let response = reqwest::blocking::get(&url).context("Failed to fetch template")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Template '{}' not found. Use 'git-ignore -l' to list available templates.",
            lang
        );
    }

    let content = response.text()?;

    fs::write(".gitignore", content).context("Failed to write .gitignore")?;

    println!("Generated .gitignore for '{}'", lang);

    Ok(())
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
