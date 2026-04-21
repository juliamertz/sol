use std::path::{Path, PathBuf};
use std::{fs, io, process};

use crate::codegen::qbe::Module;

pub fn build_ir(temp_dir: &Path, input: &Module) -> io::Result<PathBuf> {
    let text = input.to_string();
    let input_file = temp_dir.join("input.ssa");
    let output_file = temp_dir.join("out.s");

    fs::write(&input_file, &text)?;

    let mut root_cmd = process::Command::new("qbe");
    let cmd = root_cmd.arg("-o").arg(&output_file).arg(input_file);
    let child = cmd.spawn()?;
    let output = child.wait_with_output()?;

    if !output.status.success() {
        panic!("oh no");
    }

    Ok(output_file)
}

pub fn build_bin(temp_dir: &Path, ir_path: &Path) -> io::Result<PathBuf> {
    let output_file = temp_dir.join("a.out");
    let mut root_cmd = process::Command::new("cc");
    let cmd = root_cmd.arg("-o").arg(&output_file).arg(ir_path);
    let child = cmd.spawn()?;
    let output = child.wait_with_output()?;

    if !output.status.success() {
        panic!("oh no");
    }

    Ok(output_file)
}
