use std::env;
use std::io::Write;

use anyhow::Result;
use structopt::StructOpt;

use common::Git;

mod common;

#[derive(StructOpt, Debug)]
#[structopt(
bin_name = "git fork",
about = env ! ("CARGO_PKG_DESCRIPTION")
)]
struct Fork {
    branch_name: String,
    from: Option<String>,
}

fn main() -> Result<()> {
    let exit_status = execute();
    std::io::stdout().flush()?;
    std::process::exit(exit_status);
}

const SUCCESS: i32 = 0;
const FAILURE: i32 = 1;

fn execute() -> i32 {
    if let Err(err) = Fork::from_args().run() {
        eprintln!("{}", err);

        FAILURE
    } else {
        SUCCESS
    }
}

impl Fork {
    fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut git = Git::open()?;

        if git.has_file_changes()? {
            return Err(ForkError::NoCommittedChanges.into());
        }

        let branch_name = self.branch_name.as_str();
        let default_branch = git.get_default_branch("origin")?;
        let name = self
            .from
            .as_deref()
            .unwrap_or_else(|| default_branch.as_str());

        if name.contains('/') {
            git.update_upstream(name)?;
        }

        let hash_or_name = git.get_branch_hash(name)?.unwrap_or_else(
            // if name is not a branch
            || name.to_string(),
        );

        git.branch(branch_name, Some(&hash_or_name))?;

        git.switch_branch(branch_name)?;

        eprintln!("Branch {} created.", branch_name);

        Ok(())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ForkError {
    #[error("The repository has no committed changes, aborting.")]
    NoCommittedChanges,
}
