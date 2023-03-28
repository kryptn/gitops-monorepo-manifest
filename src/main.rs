use std::{collections::HashMap, fs};

use clap::Parser;
use git2::{DiffOptions, Repository, Tree};
use serde::{Deserialize, Serialize};

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
    BuildManifest {
        with_diffs: Option<String>,
        #[clap(default_value_t = String::from(".manifest.yaml"), value_parser)]
        manifest_config: String,
        head: Option<String>,
        base: Option<String>,
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

fn main() -> Result<()> {
    setup()?;

    let repo = Repository::discover(".").unwrap();
    let cli = Cli::parse();

    match cli.action {
        Action::GetDeployableRef { glob } => {}
        Action::BuildManifest {
            manifest_config,
            with_diffs,
            head,
            base,
        } => {
            let mut manifest = manifest::Manifest::new_from_path(&manifest_config)?;
            let head = match head {
                Some(v) => v,
                None => git::get_current_branch(&repo)?,
            };

            let base = match base {
                Some(v) => v,
                None => manifest.base().to_string(),
            };

            let head_sha = git::get_branch_commit_hash(&repo, &head)?;
            let merge_base_sha = git::get_merge_base(&repo, &base, &head)?;

            let diffs = match with_diffs {
                Some(given) => fs::read_to_string(given)?
                    .lines()
                    .map(|line| line.trim().to_string())
                    .collect(),
                None => git::get_changed_files(&repo, &merge_base_sha, &head_sha)?,
            };

            manifest.resolve(&diffs);

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

            println!("{}", serde_json::to_string_pretty(&out).unwrap())
        }
    }

    Ok(())
}
