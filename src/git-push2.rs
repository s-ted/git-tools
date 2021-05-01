use std::env;
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::process::Command;

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
pub struct Params {
    args: Vec<String>,
}

fn main() -> Result<()> {
    let exit_status = execute();
    std::io::stdout().flush()?;
    std::process::exit(exit_status);
}

const SUCCESS: i32 = 0;
const FAILURE: i32 = 1;

fn execute() -> i32 {
    let opts = Params::from_args();

    if let Err(err) = run(opts) {
        eprintln!("{}", err);

        FAILURE
    } else {
        SUCCESS
    }
}

pub fn run(params: Params) -> Result<(), Box<dyn std::error::Error>> {
    let git = Git::open()?;

    Err(match (git.branch_name.as_ref(), git.upstream.as_ref()) {
        (Some(name), None) => Command::new("git")
            .arg("push")
            .args(&["--set-upstream", "origin", name])
            .args(params.args)
            .exec()
            .into(),
        _ => Command::new("git")
            .arg("push")
            .args(params.args)
            .exec()
            .into(),
    })
}
