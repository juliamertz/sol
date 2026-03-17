use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum CcError {
    #[error("invalid utf8 command output")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("cc command failed")]
    Process(#[from] std::io::Error),
    #[error("build exited with status {status}")]
    Exit {
        status: ExitStatus,
        #[help]
        stderr: String,
    },
}

pub struct CcOpts<'a> {
    warnings: Option<Vec<String>>,
    include: Option<Vec<String>>,
    optimization_level: Option<u8>,
    link_time_optimization: bool,
    out_path: &'a Path,
}

impl<'a> CcOpts<'a> {
    pub fn new(out_path: &'a Path) -> Self {
        Self {
            out_path,
            warnings: None,
            include: None,
            optimization_level: None,
            link_time_optimization: false,
        }
    }

    pub fn warnings(mut self, warnings: impl IntoIterator<Item = impl ToString>) -> Self {
        self.warnings = Some(warnings.into_iter().map(|w| w.to_string()).collect());
        self
    }

    // pub fn include(mut self, include: impl IntoIterator<Item = impl ToString>) -> Self {
    //     self.warnings = Some(include.into_iter().map(|i| i.to_string()).collect());
    //     self
    // }

    pub fn link_time_optimization(mut self, value: bool) -> Self {
        self.link_time_optimization = value;
        self
    }
}

pub fn cc(source: &Path, opts: &CcOpts<'_>) -> Result<(), CcError> {
    let mut root_cmd = Command::new("cc");
    let mut cmd = root_cmd
        .env_clear()
        .stderr(Stdio::piped())
        .arg(source)
        .arg("-o")
        .arg(opts.out_path.to_string_lossy().to_string());

    if let Some(ref warnings) = opts.warnings {
        let args = warnings
            .iter()
            .map(|name| format!("-W{name}"))
            .collect::<Vec<_>>();
        cmd = cmd.args(args);
    }

    if let Some(ref include) = opts.include {
        let args = include
            .iter()
            .map(|name| format!("-I{name}"))
            .collect::<Vec<_>>();
        cmd = cmd.args(args);
    }

    if let Some(level) = opts.optimization_level {
        cmd = cmd.arg(format!("-O{level}"));
    }

    if opts.link_time_optimization {
        cmd = cmd.arg("-flto");
    }

    let output = cmd.output()?;

    if !output.status.success() {
        return Err(CcError::Exit {
            status: output.status,
            stderr: std::str::from_utf8(&output.stderr)?.to_string(),
        });
    }

    Ok(())
}
