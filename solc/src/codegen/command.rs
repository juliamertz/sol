use std::ffi::OsString;
use std::process::{Command, Stdio};
use std::rc::Rc;

use smallvec::SmallVec;

#[derive(Clone)]
pub struct CommandBuilder {
    program: Rc<OsString>,
    args: Option<SmallVec<[OsString; 4]>>,
    pipe_stdio: bool,
}

impl CommandBuilder {
    pub fn new(program: &str) -> Self {
        Self {
            program: Rc::from(OsString::from(program)),
            args: None,
            pipe_stdio: false,
        }
    }

    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        let args = self.args.get_or_insert_default();
        args.push(arg.into());
        self
    }

    pub fn with_piped_stdio(mut self) -> Self {
        self.pipe_stdio = true;
        self
    }

    pub fn build(self) -> Command {
        let mut cmd = Command::new(self.program.as_ref());
        if let Some(args) = self.args {
            cmd.args(args);
        }
        if self.pipe_stdio {
            cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        }
        cmd
    }
}
