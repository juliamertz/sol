#[derive(Debug, Clone, Copy)]
pub enum TokenKind {
    // Literals
    Int,
    String,
    Ident,

    // Operators
    Add,
    Sub,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
}

impl Token {
    pub fn new(kind: TokenKind, text: impl ToString) -> Self {
        Self {
            kind,
            text: text.to_string(),
        }
    }
}

#[derive(Debug)]
pub struct Lexer {
    pub content: String,
    pub pos: usize,
    pub curr: Option<Token>,
    pub next: Option<Token>,
}

// impl<'a> Iterator for Lexer<'a> {
//     type Item = Token;

//     fn next(&mut self) -> Option<Self::Item> {
//         self.pos += 1;

//         // TODO: clones
//         self.curr = self.next.clone();
//         self.next = self.read_token();
//         self.curr.clone()
//     }
// }

impl Lexer {
    pub fn new(content: impl ToString) -> Self {
        Self {
            content: content.to_string(),
            pos: 0,
            curr: None,
            next: None, // TODO:
        }
    }

    pub fn advance(&mut self) -> Option<char> {
        self.pos += 1;
        self.content.chars().nth(self.pos)
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
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_while<F>(&mut self, condition: F) -> &str
    where
        F: Fn(char) -> bool,
    {
        let start = self.pos;

        while let Some(ch) = self.curr() {
            if condition(ch) {
                self.advance();
            } else {
                break;
            }
        }

        &self.content[start..self.pos]
    }

    fn read_until(&mut self, until: char) -> &str {
        self.read_while(|ch| ch != until)
    }

    fn read_string(&mut self) -> &str {
        assert_eq!(self.curr(), Some('"'),);

        self.advance();
        self.read_until('"')
    }

    pub fn read_token(&mut self) -> Option<Token> {
        self.skip_whitespace();

        let token = match self.curr()? {
            '"' => Token::new(TokenKind::String, self.read_string().to_string()),
            '+' => Token::new(TokenKind::Add, "+"),
            '-' => Token::new(TokenKind::Sub, "-"),
            ch if ch.is_ascii_digit() => {
                let text = self.read_while(|ch| ch.is_ascii_digit());
                return Some(Token::new(TokenKind::Int, text));
            }
            // ch if ch.is_ascii_alphabetic() || ch == '_' => { },
            _ => return None,
        };

        self.advance();
        Some(token)
    }
}
