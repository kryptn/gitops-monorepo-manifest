use std::io::Write;
use std::{collections::HashMap, fs::OpenOptions};

use clap::Parser;
use git2::Repository;
use serde::Serialize;

use anyhow::Result;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

mod git;
mod manifest;

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    action: Action,

    #[clap(default_value_t = String::from("."), value_parser)]
    repo_path: String,
}

#[derive(clap::Subcommand, Debug)]
enum Action {
    GetDeployableRef {
        glob: String,
    },
    ShowBranches,
    Derive {
        #[clap(long, default_value_t = String::from(".manifest.yaml"), value_parser)]
        config: String,
        #[clap(long)]
        /// Defaults to the current branch
        head: Option<String>,
        #[clap(long)]
        /// Defaults to the base defined in the manifest config
        base: Option<String>,
        /// Forces all services to be activated
        #[clap(long, short, action = clap::ArgAction::Count)]
        force: u8,
        /// Apply force if building on base
        #[clap(long, default_value_t = false)]
        force_on_base: bool,
        /// Write manifest into github actions output
        #[clap(long, default_value_t = false)]
        actions_output: bool,
        /// Write manifest summary into github actions step summary
        #[clap(long, default_value_t = false)]
        step_summary: bool,
    },
}

fn setup() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    Ok(())
}

#[derive(Serialize)]
struct TargetOutput {
    changed: bool,
    sha: String,
}

fn write_output(output: &HashMap<String, TargetOutput>) -> Result<()> {
    let (mut changed, mut unchanged): (Vec<_>, Vec<_>) =
        output.keys().partition(|k| output[*k].changed);
    changed.sort();
    unchanged.sort();

    let output_file = std::env::var("GITHUB_OUTPUT")?;
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(output_file)?;

    let manifest = serde_json::to_string(&output)?;
    let changed_str = serde_json::to_string(&changed)?;
    let unchanged_str = serde_json::to_string(&unchanged)?;

    writeln!(file, "manifest='{}'", manifest)?;
    writeln!(file, "changed_targets='{}'", changed_str)?;
    writeln!(file, "unchanged_targets='{}'", unchanged_str)?;

    Ok(())
}

fn write_summary(output: &HashMap<String, TargetOutput>) -> Result<()> {
    let raw_summary = format!(
        "```json\n{}\n```",
        serde_json::to_string_pretty(&output).unwrap()
    );

    let summary_file = std::env::var("GITHUB_STEP_SUMMARY")?;
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(summary_file)?;

    writeln!(file, "{}", raw_summary)?;

    Ok(())
}

fn main() -> Result<()> {
    setup()?;

    let repo = Repository::discover(".").unwrap();
    let cli = Cli::parse();

    let current_branch = git::get_current_branch(&repo)?;
    let latest_commit = git::get_branch_commit_hash(&repo, &current_branch)?;

    eprintln!("{current_branch:?}, {latest_commit:?}");

    match cli.action {
        Action::GetDeployableRef { glob: _ } => {}
        Action::ShowBranches => {
            let branches = git::get_all_branches(&repo, None)?;
            println!("no filter: \n{}\n", branches.join("\n"));
            let branches = git::get_all_branches(&repo, Some(git2::BranchType::Local))?;
            println!("local filter: \n{}\n", branches.join("\n"));
            let branches = git::get_all_branches(&repo, Some(git2::BranchType::Remote))?;
            println!("remote filter: \n{}\n", branches.join("\n"));
        }
        Action::Derive {
            config,
            head,
            base,
            force,
            actions_output,
            step_summary,
            force_on_base,
        } => {
            let mut manifest = manifest::Manifest::new_from_path(&config)?;
            let head = match head {
                Some(v) => v,
                None => git::get_current_branch(&repo)?,
            };

            let base = match base {
                Some(v) => v,
                None => manifest.base().to_string(),
            };

            let force = if force_on_base && manifest.base() == &head {
                force + 1
            } else {
                force
            };

            let head_sha = git::get_branch_commit_hash(&repo, &head)?;
            let merge_base_sha = git::get_merge_base(&repo, &base, &head)?;

            let diffs = git::get_changed_files(&repo, &merge_base_sha, &head_sha)?;

            manifest.resolve(&diffs, force);

            let out: HashMap<String, TargetOutput> = manifest
                .activated_targets()
                .iter()
                .map(|(t, a)| {
                    let sha = if *a { &head_sha } else { &merge_base_sha };
                    let op = TargetOutput {
                        changed: *a,
                        sha: sha.clone(),
                    };

                    (t.clone(), op)
                })
                .collect();

            if actions_output {
                write_output(&out)?;
            }

            if step_summary {
                write_summary(&out)?;
            }
        }
    }

    Ok(())
}
