use std::ffi::OsString;
use std::process::{Command, Stdio};
use std::rc::Rc;

use smallvec::SmallVec;

// this is silly but i wanted to mess around with bits.

#[derive(Clone, Copy)]
struct FlagId(u8);

const STDOUT: FlagId = FlagId(0);
const STDERR: FlagId = FlagId(1);

#[derive(Default, Clone, Copy)]
struct PipeFlags(u8);

impl PipeFlags {
    pub fn toggle(&mut self, FlagId(idx): FlagId) {
        self.0 ^= 1 << idx
    }

    pub fn is_set(&self, FlagId(idx): FlagId) -> bool {
        self.0 & (1 << idx) != 0
    }

    #[allow(dead_code)]
    pub fn is_unset(&self, flag_id: FlagId) -> bool {
        !self.is_set(flag_id)
    }

    pub fn set_on(&mut self, id: FlagId) {
        if !self.is_set(id) {
            self.toggle(id);
        }
    }
}

#[derive(Clone)]
pub struct CommandBuilder {
    program: Rc<OsString>,
    args: Option<SmallVec<[OsString; 4]>>,
    pipe_flags: PipeFlags,
}

impl CommandBuilder {
    pub fn new(program: &str) -> Self {
        Self {
            program: Rc::from(OsString::from(program)),
            args: None,
            pipe_flags: PipeFlags::default(),
        }
    }

    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        let args = self.args.get_or_insert_default();
        args.push(arg.into());
        self
    }

    pub fn with_piped_stdio(mut self) -> Self {
        self.pipe_flags.set_on(STDOUT);
        self.pipe_flags.set_on(STDERR);
        self
    }

    pub fn build(self) -> Command {
        let mut cmd = Command::new(self.program.as_ref());
        if let Some(args) = self.args {
            cmd.args(args);
        }
        if self.pipe_flags.is_set(STDOUT) {
            cmd.stdout(Stdio::piped());
        }
        if self.pipe_flags.is_set(STDERR) {
            cmd.stderr(Stdio::piped());
        }
        cmd
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn pipeflags() {
        let mut flags = PipeFlags::default();

        assert!(flags.is_unset(STDOUT));
        assert!(flags.is_unset(STDERR));

        flags.toggle(STDOUT);

        assert!(flags.is_set(STDOUT));
        assert!(flags.is_unset(STDERR));

        flags.toggle(STDERR);

        assert!(flags.is_set(STDOUT));
        assert!(flags.is_set(STDERR));
    }
}
