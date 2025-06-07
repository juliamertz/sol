use miette::SourceSpan;

#[derive(Debug)]
pub struct Loc<'a> {
    source: &'a str,
    pos: usize,
    line: usize,
    column: usize,
}

impl<'a> Loc<'a> {
    fn walk(&mut self) {
        let chars = self.source.chars().take(self.pos + 1).collect::<Vec<_>>();
        dbg!(&chars);
        for ch in chars {
            if ch == '\n' {
                self.line += 1;
                self.column = 0;
            } else {
                self.column += 1;
            }
        }
    }

    pub fn new(source: &'a str, pos: usize) -> Self {
        let mut loc = Self {
            source,
            pos,
            line: 1,
            column: 1,
        };

        loc.walk();
        loc
    }

    pub fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
        self.line = 1;
        self.column = 1;
        self.walk();
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn column(&self) -> usize {
        self.column
    }
}

impl <'a> Into<SourceSpan> for Loc<'a> {
    fn into(self) -> SourceSpan {
        (self.line(), self.column()).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let src = vec!["hey young world", "the world is yours"].join("\n");

        let loc = Loc::new(&src, 16);
        panic!("{loc:?}")
    }
}
