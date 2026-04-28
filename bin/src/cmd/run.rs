use miette::{IntoDiagnostic, Result};

use crate::cmd::build::{BuildOpts, build};

pub fn handle(opts: BuildOpts) -> Result<()> {
    let bin_path = build(&opts)?;
    let _out = std::process::Command::new(&bin_path)
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    if opts.cleanup {
        std::fs::remove_file(bin_path).into_diagnostic()?;
    }

    Ok(())
}
