use std::path::{Path, PathBuf};
use std::{fs, io};

use miette::Diagnostic;
use thiserror::Error;

use crate::codegen::command::CommandBuilder;
use crate::codegen::qbe::Module;
use crate::codegen::temp_dir::TempDir;

#[derive(Debug, Error, Diagnostic)]
pub enum CompileError {
    #[error("i/o error: {0}")]
    Io(#[from] io::Error),
    #[error("command output contains non-utf8 bytes: {0}")]
    StringFromUtf8(#[from] std::string::FromUtf8Error),
    #[error("process exited: {status}")]
    Exit {
        status: std::process::ExitStatus,
        #[help]
        stderr: String,
    },
}

pub struct Compiler {
    qbe: CommandBuilder,
    cc: CommandBuilder,
    temp_dir: TempDir,
    output_dir: PathBuf,
}

impl Compiler {
    pub fn new(output_dir: impl Into<PathBuf>) -> Self {
        Self {
            qbe: CommandBuilder::new("qbe").with_piped_stdio(),
            cc: CommandBuilder::new("cc").with_piped_stdio(),
            temp_dir: TempDir::new(),
            output_dir: output_dir.into(),
        }
    }

    /// build assembly instructions from a qbe ir
    pub fn ir_to_asm(&self, input: &Module) -> Result<PathBuf, CompileError> {
        let text = input.to_string();
        let input_file = self.temp_dir.path().join("input.ssa");
        let output_file = self.temp_dir.path().join("out.s");

        fs::write(&input_file, &text)?;

        let cmd = self.qbe.clone();
        let child = cmd
            .arg("-o")
            .arg(&output_file)
            .arg(input_file)
            .build()
            .spawn()?;

        let output = child.wait_with_output()?;

        if !output.status.success() {
            let stderr = String::from_utf8(output.stderr)?;
            Err(CompileError::Exit {
                status: output.status,
                stderr,
            })
        } else {
            Ok(output_file)
        }
    }

    /// build binary executable from asm generated with QBE backend
    pub fn asm_to_bin(&self, asm_path: &Path) -> Result<PathBuf, CompileError> {
        let output_file = self.output_dir.join("a.out");
        let cmd = self.cc.clone();
        let child = cmd
            .arg("-o")
            .arg(&output_file)
            .arg(asm_path)
            .build()
            .spawn()?;

        let output = child.wait_with_output()?;
        if !output.status.success() {
            let stderr = String::from_utf8(output.stderr)?;
            Err(CompileError::Exit {
                status: output.status,
                stderr,
            })
        } else {
            Ok(output_file)
        }
    }

    pub fn ir_to_bin(&self, input: &Module) -> Result<PathBuf, CompileError> {
        let asm_path = self.ir_to_asm(input)?;
        self.asm_to_bin(&asm_path)
    }
}
