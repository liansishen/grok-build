use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use clap::Subcommand;
use xai_grok_i18n::{t, t_fmt};
use xai_grok_shell::session::memory::storage::MemoryStorage;

#[derive(Debug, clap::Args, Clone)]
pub struct MemoryArgs {
    #[command(subcommand)]
    pub command: MemoryCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum MemoryCommand {
    #[command(about = t("cli.memory.help.clear"))]
    Clear {
        #[arg(
            long,
            group = "scope",
            help = t("cli.memory.help.workspace")
        )]
        workspace: bool,
        #[arg(long, group = "scope", help = t("cli.memory.help.global"))]
        global: bool,
        #[arg(long, group = "scope", help = t("cli.memory.help.all"))]
        all: bool,
        #[arg(long, short = 'y', help = t("cli.memory.help.yes"))]
        yes: bool,
    },
}

struct ClearTarget {
    label_key: &'static str,
    path: PathBuf,
    clear: fn(&MemoryStorage) -> std::io::Result<bool>,
}

fn workspace_target(storage: &MemoryStorage) -> ClearTarget {
    ClearTarget {
        label_key: "cli.memory.workspace_memory",
        path: storage.workspace_dir().to_path_buf(),
        clear: |s| s.clear_workspace(),
    }
}

fn global_target(storage: &MemoryStorage) -> ClearTarget {
    ClearTarget {
        label_key: "cli.memory.global_memory",
        path: storage.global_memory_file(),
        clear: |s| s.clear_global(),
    }
}

pub fn run(args: MemoryArgs) -> Result<()> {
    match args.command {
        MemoryCommand::Clear {
            global, all, yes, ..
        } => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
            let storage = MemoryStorage::new(&cwd, None);

            let targets = if all {
                vec![workspace_target(&storage), global_target(&storage)]
            } else if global {
                vec![global_target(&storage)]
            } else {
                vec![workspace_target(&storage)]
            };

            run_clear(&storage, &targets, yes)
        }
    }
}

fn run_clear(storage: &MemoryStorage, targets: &[ClearTarget], skip_confirm: bool) -> Result<()> {
    let existing: Vec<_> = targets.iter().filter(|t| t.path.exists()).collect();

    if existing.is_empty() {
        println!("{}", t("cli.memory.nothing_to_clear"));
        return Ok(());
    }

    println!("{}", t("cli.memory.will_delete"));
    for target in &existing {
        println!(
            "{}",
            t_fmt(
                "cli.memory.target_path",
                &[
                    ("label", t(target.label_key)),
                    ("path", target.path.to_string_lossy().as_ref()),
                ],
            )
        );
    }

    if !skip_confirm {
        print!("\n{}", t("cli.memory.confirm"));
        std::io::stdout().flush().map_err(|error| {
            anyhow::anyhow!(t_fmt(
                "cli.memory.stdout_failed",
                &[("error", error.to_string().as_str())]
            ))
        })?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input).map_err(|error| {
            anyhow::anyhow!(t_fmt(
                "cli.memory.stdin_failed",
                &[("error", error.to_string().as_str())]
            ))
        })?;
        if !matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
            println!("{}", t("cli.memory.cancelled"));
            return Ok(());
        }
    }

    let mut cleared = false;
    let mut errors: Vec<String> = Vec::new();
    for target in targets {
        match (target.clear)(storage) {
            Ok(true) => {
                cleared = true;
                println!(
                    "{}",
                    t_fmt(
                        "cli.memory.cleared_target",
                        &[("label", t(target.label_key))]
                    )
                );
            }
            Ok(false) => {} // nothing to clear for this scope
            Err(error) => {
                errors.push(t_fmt(
                    "cli.memory.target_error",
                    &[
                        ("label", t(target.label_key)),
                        ("error", error.to_string().as_str()),
                    ],
                ));
            }
        }
    }

    if cleared && errors.is_empty() {
        println!("{}", t("cli.memory.cleared"));
    } else if cleared {
        println!("{}", t("cli.memory.partially_cleared"));
        for error in &errors {
            eprintln!("  {error}");
        }
    } else if !errors.is_empty() {
        eprintln!("{}", t("cli.memory.clear_failed"));
        for error in &errors {
            eprintln!("  {error}");
        }
        return Err(anyhow::anyhow!(t("cli.memory.clear_failed_error")));
    }

    Ok(())
}
