use std::env;
use std::io::Write;
use std::process::{Command, ExitStatus};

use anyhow::Result;
use structopt::{clap::AppSettings, StructOpt};

use common::Git;

mod common;

#[derive(StructOpt, Debug)]
#[structopt(
bin_name = "git push2",
about = env ! ("CARGO_PKG_DESCRIPTION"),
settings = & [AppSettings::TrailingVarArg, AppSettings::AllowLeadingHyphen],
)]
struct Push2 {
    args: Vec<String>,
}

fn main() -> Result<()> {
    let exit_status = Push2::from_args().run()?;
    std::io::stdout().flush()?;

    std::process::exit(exit_status.code().unwrap_or_default())
}

impl Push2 {
    fn run(&self) -> Result<ExitStatus> {
        let git = Git::open()?;

        let upstream_args = match (git.branch_name.as_ref(), git.upstream.as_ref()) {
            (Some(name), None) => vec!["--set-upstream", "origin", name],
            _ => vec![],
        };

        Ok(Command::new("git")
            .arg("push")
            .args(&upstream_args)
            .args(&self.args)
            .spawn()?
            .wait()?)
    }
}
