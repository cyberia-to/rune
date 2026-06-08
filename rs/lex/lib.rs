/// Token stream for rune. Both classic and pure registers share this lexer.
/// The parser disambiguates register by context (file pragma or fenced block tag).
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Int(u64),
    Float(String),
    Str(String),
    Ident(String),
    // Sigils — atomic semantics defined in cybermark
    Hash,   // # — particle
    Pat,    // @ — neuron / identity
    Sig,    // ~ — name / annotation
    Fas,    // / — scope / path
    Buc,    // $ — token / economic
    Ket,    // ^ — abstract / lift
    Zap,    // ! — effect / action
    Dot,    // . — transform / apply
    Bar,    // | — core / composition
    Tis,    // = — bind / equivalence
    Wut,    // ? — test / decision
    Col,    // : — pair / two-way
    Lus,    // + — augment
    // Classic register keywords
    Fn, Let, If, Else, Loop, Return, Struct, Enum, Impl, Pub, Use, Async, Await,
    // Punctuation
    LParen, RParen, LBrace, RBrace, LBrack, RBrack,
    Comma, Semi, Bang, Amp, Pipe, Percent, Star, Slash,
    Arrow,      // ->
    FatArrow,   // =>
    ColCol,     // ::
    DotDot,     // ..
    // Layout (significant in pure register tall form)
    Newline,
    Indent,
    Dedent,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub inner: T,
    pub span: Span,
}

pub struct Lexer<'src> {
    src: &'src str,
    pos: usize,
}

impl<'src> Lexer<'src> {
    pub fn new(src: &'src str) -> Self {
        Lexer { src, pos: 0 }
    }

    pub fn tokenize(mut self) -> Vec<Spanned<Token>> {
        let mut out = Vec::new();
        loop {
            self.skip_horizontal_whitespace();
            let start = self.pos;
            match self.next_token() {
                Some(tok) => out.push(Spanned { inner: tok, span: Span { start, end: self.pos } }),
                None if self.pos >= self.src.len() => break,
                None => { self.pos += 1; } // skip unknown char
            }
        }
        out.push(Spanned { inner: Token::Eof, span: Span { start: self.pos, end: self.pos } });
        out
    }

    fn next_token(&mut self) -> Option<Token> {
        // Skip line comments
        if self.peek2() == Some((':', ':')) {
            while !matches!(self.peek(), Some('\n') | None) { self.advance(); }
            return None;
        }
        // Skip block comments `/* ... */`
        if self.peek2() == Some(('/', '*')) {
            self.advance(); self.advance();
            while self.peek2() != Some(('*', '/')) && self.pos < self.src.len() { self.advance(); }
            self.advance(); self.advance();
            return None;
        }

        let ch = self.peek()?;
        Some(match ch {
            '\n' => { self.advance(); Token::Newline }
            '#' => { self.advance(); Token::Hash }
            '@' => { self.advance(); Token::Pat }
            '~' => { self.advance(); Token::Sig }
            '/' => { self.advance(); Token::Fas }
            '$' => { self.advance(); Token::Buc }
            '^' => { self.advance(); Token::Ket }
            '!' => { self.advance(); Token::Zap }
            '.' => {
                self.advance();
                if self.peek() == Some('.') { self.advance(); Token::DotDot }
                else { Token::Dot }
            }
            '|' => { self.advance(); Token::Bar }
            '=' => {
                self.advance();
                if self.peek() == Some('>') { self.advance(); Token::FatArrow }
                else { Token::Tis }
            }
            '?' => { self.advance(); Token::Wut }
            ':' => {
                self.advance();
                if self.peek() == Some(':') { self.advance(); Token::ColCol }
                else { Token::Col }
            }
            '+' => { self.advance(); Token::Lus }
            '-' => {
                self.advance();
                if self.peek() == Some('>') { self.advance(); Token::Arrow }
                else { Token::Ident("-".into()) }
            }
            '(' => { self.advance(); Token::LParen }
            ')' => { self.advance(); Token::RParen }
            '{' => { self.advance(); Token::LBrace }
            '}' => { self.advance(); Token::RBrace }
            '[' => { self.advance(); Token::LBrack }
            ']' => { self.advance(); Token::RBrack }
            ',' => { self.advance(); Token::Comma }
            ';' => { self.advance(); Token::Semi }
            '&' => { self.advance(); Token::Amp }
            '%' => { self.advance(); Token::Percent }
            '*' => { self.advance(); Token::Star }
            '<' => {
                self.advance();
                if self.peek() == Some('=') { self.advance(); Token::Ident("<=".into()) }
                else { Token::Ident("<".into()) }
            }
            '>' => {
                self.advance();
                if self.peek() == Some('=') { self.advance(); Token::Ident(">=".into()) }
                else { Token::Ident(">".into()) }
            }
            '"' => self.lex_str(),
            '0'..='9' => self.lex_number(),
            'a'..='z' | 'A'..='Z' | '_' => self.lex_ident(),
            _ => return None,
        })
    }

    fn lex_number(&mut self) -> Token {
        let start = self.pos;
        while matches!(self.peek(), Some('0'..='9' | '_')) { self.advance(); }
        if self.peek() == Some('.') && matches!(self.peek_at(1), Some('0'..='9')) {
            self.advance();
            while matches!(self.peek(), Some('0'..='9' | '_')) { self.advance(); }
            Token::Float(self.src[start..self.pos].to_string())
        } else {
            let s: String = self.src[start..self.pos].chars().filter(|c| *c != '_').collect();
            Token::Int(s.parse().unwrap_or(0))
        }
    }

    fn lex_str(&mut self) -> Token {
        self.advance();
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c != '"') {
            if self.peek() == Some('\\') { self.advance(); }
            self.advance();
        }
        let s = self.src[start..self.pos].to_string();
        if self.peek() == Some('"') { self.advance(); }
        Token::Str(s)
    }

    fn lex_ident(&mut self) -> Token {
        let start = self.pos;
        while matches!(self.peek(), Some('a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-')) {
            self.advance();
        }
        match &self.src[start..self.pos] {
            "fn"     => Token::Fn,
            "let"    => Token::Let,
            "if"     => Token::If,
            "else"   => Token::Else,
            "loop"   => Token::Loop,
            "return" => Token::Return,
            "struct" => Token::Struct,
            "enum"   => Token::Enum,
            "impl"   => Token::Impl,
            "pub"    => Token::Pub,
            "use"    => Token::Use,
            "async"  => Token::Async,
            "await"  => Token::Await,
            word     => Token::Ident(word.to_string()),
        }
    }

    fn skip_horizontal_whitespace(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\r')) { self.advance(); }
    }

    fn peek(&self) -> Option<char> { self.src[self.pos..].chars().next() }
    fn peek_at(&self, offset: usize) -> Option<char> {
        self.src[self.pos..].chars().nth(offset)
    }
    fn peek2(&self) -> Option<(char, char)> {
        let mut it = self.src[self.pos..].chars();
        Some((it.next()?, it.next()?))
    }
    fn advance(&mut self) {
        if let Some(c) = self.src[self.pos..].chars().next() {
            self.pos += c.len_utf8();
        }
    }
}
