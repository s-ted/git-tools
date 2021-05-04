use std::env;

use anyhow::{Context, Result};
use structopt::StructOpt;

mod common;

#[derive(StructOpt, Debug)]
#[structopt(
bin_name = "git delete",
about = env ! ("CARGO_PKG_DESCRIPTION")
)]
struct Delete {
    branch_name: String,
}

fn main() {
    let exit_status = execute();
    std::process::exit(exit_status);
}

const SUCCESS: i32 = 0;
const FAILURE: i32 = 1;

fn execute() -> i32 {
    if let Err(err) = Delete::from_args().run() {
        eprintln!("{}", err);

        FAILURE
    } else {
        SUCCESS
    }
}

impl Delete {
    fn run(&self) -> Result<()> {
        let repo = git2::Repository::open(".").context("Could not open repository")?;

        let mut branch = repo
            .find_branch(&self.branch_name, git2::BranchType::Local)
            .with_context(|| format!("Could not find local branch: {}", self.branch_name))?;

        let branch_name = branch
            .name()
            .context("Could not retrieve branch name")?
            .context("not valid utf-8")?
            .to_owned();

        if branch.is_head() {
            return Err(GitDeleteError::DeletingCurrentHead.into());
        }

        // delete remote branch if any
        if let Ok(upstream) = branch.upstream() {
            let upstream_name = upstream.get().name().context("not valid utf-8")?;
            let remote_name = upstream_name
                .strip_suffix(&branch_name)
                .and_then(|x| x.strip_prefix("refs/remotes/"))
                .context("Could not find remote name")?
                .trim_end_matches('/');

            let mut remote = repo
                .find_remote(remote_name)
                .with_context(|| format!("Could not find remote `{}`", remote_name))?;

            // this is a reference to the default branch if it exists
            let head_reference = repo.find_reference(&format!("refs/remotes/{}/HEAD", remote_name));

            let default_branch_name = {
                match head_reference {
                    Ok(reference) => reference
                        .symbolic_target()
                        .context("Invalid reference HEAD: not symbolic reference")?
                        .to_string(),
                    Err(err) if err.code() == git2::ErrorCode::NotFound => {
                        format!("refs/remote/{}/master", remote_name)
                    }
                    Err(err) => return Err(GitDeleteError::UnableToFindCurrentBranch(err).into()),
                }
            };

            if upstream_name == default_branch_name {
                return Err(GitDeleteError::DeletingDefaultBranch.into());
            }

            // TODO better handling for credentials using git2_credentials
            //      make sure it works with ~/.ssh/id_rsa and ssh-agent
            let mut remote_callbacks = git2::RemoteCallbacks::new();
            let mut handler = common::CredentialHandler::new()?;
            remote_callbacks.credentials(move |url, username_from_url, allowed_types| {
                handler
                    .credentials_callback(url, username_from_url, allowed_types)
                    .map_err(|e| git2::Error::from_str(&e.to_string()))
            });

            let mut push_options = git2::PushOptions::new();
            push_options.remote_callbacks(remote_callbacks);

            remote.push(
                &[&format!("+:refs/heads/{}", branch_name)],
                Some(&mut push_options),
            )?;
            eprintln!("Upstream deleted: {}", upstream_name);
        }

        branch.delete()?;
        eprintln!("Local branch deleted: {}", branch_name);

        Ok(())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum GitDeleteError {
    #[error("Aborted: deleting default branch is forbidden")]
    DeletingDefaultBranch,

    #[error("Aborted: cannot delete branch currently pointed at by HEAD")]
    DeletingCurrentHead,

    #[error("Could not find default branch for this repository")]
    UnableToFindCurrentBranch(#[source] git2::Error),
}
