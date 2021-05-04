use std::collections::HashSet;
use std::io::Write;
use std::process::Command;

use anyhow::{Context, Result};
use globset::{Glob, GlobSetBuilder};
use structopt::{clap::AppSettings, StructOpt};

use common::Git;

mod common;

#[derive(StructOpt, Debug)]
#[structopt(
bin_name = "git try-merge",
about = env ! ("CARGO_PKG_DESCRIPTION"),
settings = & [AppSettings::TrailingVarArg, AppSettings::AllowLeadingHyphen],
)]
struct TryMerge {
    /// Squash all the merge commits together at the end.
    ///
    /// You can make this behavior the default using the following command:
    ///
    /// git config --global try-merge.squash true
    ///
    /// Or for this repository only:
    ///
    /// git config try-merge.squash true
    #[structopt(long)]
    squash: bool,

    // NOTE: the long and short name for the parameters must not conflict with `git merge`
    /// Do not run `git merge` at the end. (Merge to the latest commit possible without conflict.)
    #[structopt(long, short = "u")]
    no_merge: bool,

    /// Revision for the update (default branch or origin/main by default).
    revision: Option<String>,

    merge_args: Vec<String>,
}

fn main() -> Result<()> {
    let exit_status = TryMerge::from_args().run()?;
    std::io::stdout().flush()?;

    std::process::exit(exit_status)
}

const SUCCESS: i32 = 0;

impl TryMerge {
    pub fn run(self: TryMerge) -> Result<i32> {
        let mut git = Git::open()?;

        let top_rev = if let Some(revision) = self.revision {
            revision
        } else {
            git.get_default_branch("origin")?
        };

        if top_rev.contains('/') {
            git.update_upstream(top_rev.as_str())?;
        }

        if git.has_file_changes()? {
            return Err(TryMergeError::NoCommittedChanges.into());
        }

        let mut rev_list = git.rev_list("HEAD", top_rev.as_str(), true)?;

        if rev_list.is_empty() {
            let default_squash = git.config.get_bool("try-merge.squash").ok();
            if self.squash || default_squash.unwrap_or_default() {
                let commit = Self::squash_all_merge_commits(&mut git, &top_rev)?;
                if commit.is_some() {
                    eprintln!("Your merge commits have been squashed.");
                    return Ok(SUCCESS);
                }
            }
            eprintln!("Your branch is already up-to-date.");
            return Ok(SUCCESS);
        }

        let mut builder = GlobSetBuilder::new();
        for entry in git
            .config
            .multivar("try-merge.ignore-conflict", None)
            .iter()
            .flatten()
            .filter_map(|x| x.ok())
        {
            builder.add(Glob::new(entry.value().context("invalid UTF-8")?)?);
        }
        let ignore_conflict_set = builder.build()?;

        let mut skipped = 0;
        let mut last_failing_revision: Option<String> = None;
        let mut all_ignored_conflicts = HashSet::new();
        while let Some(revision) = rev_list.pop() {
            let message = format!("Merge commit {} (no conflict)\n\n", revision,);

            if let Some((_, ignored_conflicts)) =
                git.merge_no_conflict(revision.as_str(), message.as_str(), &ignore_conflict_set)?
            {
                println!(
                    "All the commits to {} have been merged successfully without conflict",
                    revision
                );
                all_ignored_conflicts.extend(ignored_conflicts);

                break;
            } else {
                skipped += 1;
                last_failing_revision = Some(revision.clone());
            }
        }

        if !all_ignored_conflicts.is_empty() {
            println!("The following files had conflicts but have been ignored:");
            for file_path in all_ignored_conflicts {
                println!("{}", file_path);
            }
        }

        if self.no_merge {
            return Ok(SUCCESS);
        } else if let Some(revision) = last_failing_revision {
            println!(
                "Your current branch is still behind '{}' by {} commit(s).",
                top_rev, skipped
            );
            println!("First merge conflict detected on: {}", revision);

            let message = format!("Merge commit {} (conflicts)\n\n", revision,);

            return Ok(Command::new("git")
                .args(&[
                    "merge",
                    "--no-ff",
                    revision.as_str(),
                    "-m",
                    message.as_str(),
                ])
                .args(self.merge_args)
                .spawn()?
                .wait()?
                .code()
                .unwrap_or(SUCCESS));
        } else {
            println!("Nothing more to merge. Your branch is up-to-date.");
        }

        Ok(SUCCESS)
    }

    fn squash_all_merge_commits(git: &mut Git, top_rev: &str) -> Result<Option<String>> {
        let merge_commits = git.ancestors("HEAD")?.take_while(|commit| {
            commit
                .message()
                .map(|msg| msg.starts_with("Merge commit"))
                .unwrap_or_default()
        });
        if let Some(ancestor) = merge_commits
            // NOTE: we need to have more than 1 commit to make a squash
            .skip(1)
            .last()
            .map(|x| {
                x.parent(0)
                    .map(|parent| format!("{}", parent.id()))
            })
            .transpose()?
        {
            Ok(Some(git.squash(
                &ancestor,
                top_rev,
                &format!("Merge branch {}", top_rev),
            )?))
        } else {
            Ok(None)
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum TryMergeError {
    #[error("The repository has no committed changes, aborting.")]
    NoCommittedChanges,
}
