#[derive(Debug)]
pub enum Token {
    // Literals
    Int(i64),
    String(String),
    Ident(String),

    // Operators
    Add,
    Sub,
}

#[derive(Debug)]
pub struct Lexer<'a> {
    content: &'a str,
    pos: usize,
    curr: char,
    next: char,
}

impl<'a> Iterator for Lexer<'a> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        self.pos += 1;

        self.curr = self.next;
        self.next = self.content.chars().nth(self.pos + 1)?;
        Some(self.next)
    }
}

impl<'a> Lexer<'a> {
    pub fn new(content: &'a str) -> Self {
        Self {
            content,
            pos: 0,
            curr: content.chars().nth(0).unwrap(),
            next: content.chars().nth(1).unwrap(), // TODO:
        }
    }

    pub fn curr(&self) -> Option<char> {
        self.content.chars().nth(self.pos)
    }

    pub fn peek(&self) -> Option<char> {
        self.content.chars().nth(self.pos + 1)
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.curr() {
            if ch.is_ascii_whitespace() {
                self.next();
            } else {
                break;
            }
        }
    }

    // TODO: here for debugging purposes
    pub fn remaining(&self) -> &str {
        &self.content[self.pos..self.content.len()]
    }

    fn read_while<F>(&mut self, condition: F) -> &str
    where
        F: Fn(char) -> bool,
    {
        let start = self.pos;

        while let Some(ch) = self.curr() {
            if condition(ch) {
                self.next();
            } else {
                break;
            }
        }

        &self.content[start..self.pos]
    }

    // TODO: code duplication
    fn read_while_peeked<F>(&mut self, condition: F) -> &str
    where
        F: Fn(char) -> bool,
    {
        let start = self.pos;

        while let Some(ch) = self.peek() {
            if condition(ch) {
                self.next();
            } else {
                break;
            }
        }

        &self.content[start..self.pos]
    }

    fn read_until(&mut self, until: char) -> &str {
        self.read_while(|ch| ch != until)
    }

    pub fn read_string(&mut self) -> &str {
        assert_eq!(self.curr(), Some('"'),);

        self.next();
        self.read_until('"')
    }

    pub fn read_token(&mut self) -> Option<Token> {
        self.skip_whitespace();

        let token = match self.curr()? {
            '"' => Token::String(self.read_string().to_string()),
            '+' => Token::Add,
            '-' => Token::Sub,
            ch if ch.is_ascii_digit() => {
                let text = self.read_while_peeked(|ch| ch.is_ascii_digit());
                Token::Int(text.parse().unwrap())
            }
            // ch if ch.is_ascii_alphabetic() || ch == '_' => { },
            _ => return None,
        };

        self.next();
        Some(token)
    }
}
