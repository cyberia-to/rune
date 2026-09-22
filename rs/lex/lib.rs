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
                // Comment-skipping and the unknown-char arm below already advance
                // `pos` past whatever they consumed; a second blind `pos += 1` here
                // double-skips (dropping the char right after a comment) and, for a
                // multi-byte unknown char, lands mid-character and panics on the
                // next slice.
                None => {}
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
            _ => { self.advance(); return None; }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(src: &str) -> Vec<Token> {
        Lexer::new(src).tokenize().into_iter().map(|s| s.inner).collect()
    }

    #[test]
    fn eof_always_terminates() {
        assert_eq!(toks(""), vec![Token::Eof]);
    }

    #[test]
    fn sigils() {
        assert_eq!(
            toks("#@~/$^!|?:+"),
            vec![
                Token::Hash, Token::Pat, Token::Sig, Token::Fas, Token::Buc, Token::Ket,
                Token::Zap, Token::Bar, Token::Wut, Token::Col, Token::Lus, Token::Eof,
            ]
        );
    }

    #[test]
    fn dot_vs_dotdot() {
        assert_eq!(toks("."), vec![Token::Dot, Token::Eof]);
        assert_eq!(toks(".."), vec![Token::DotDot, Token::Eof]);
    }

    #[test]
    fn equals_vs_fat_arrow() {
        assert_eq!(toks("="), vec![Token::Tis, Token::Eof]);
        assert_eq!(toks("=>"), vec![Token::FatArrow, Token::Eof]);
    }

    #[test]
    fn colon_vs_coloncolon() {
        assert_eq!(toks(":"), vec![Token::Col, Token::Eof]);
        assert_eq!(toks("::x"), vec![Token::Eof]); // `::` opens a line comment, not ColCol
    }

    #[test]
    fn minus_vs_arrow() {
        assert_eq!(toks("-"), vec![Token::Ident("-".into()), Token::Eof]);
        assert_eq!(toks("->"), vec![Token::Arrow, Token::Eof]);
    }

    #[test]
    fn comparison_operators() {
        assert_eq!(toks("<"), vec![Token::Ident("<".into()), Token::Eof]);
        assert_eq!(toks("<="), vec![Token::Ident("<=".into()), Token::Eof]);
        assert_eq!(toks(">"), vec![Token::Ident(">".into()), Token::Eof]);
        assert_eq!(toks(">="), vec![Token::Ident(">=".into()), Token::Eof]);
    }

    #[test]
    fn punctuation() {
        assert_eq!(
            toks("(){}[],;&%*"),
            vec![
                Token::LParen, Token::RParen, Token::LBrace, Token::RBrace,
                Token::LBrack, Token::RBrack, Token::Comma, Token::Semi,
                Token::Amp, Token::Percent, Token::Star, Token::Eof,
            ]
        );
    }

    #[test]
    fn newline_token() {
        assert_eq!(toks("\n"), vec![Token::Newline, Token::Eof]);
    }

    #[test]
    fn horizontal_whitespace_is_not_tokenized() {
        assert_eq!(toks("  \t \r "), vec![Token::Eof]);
    }

    #[test]
    fn classic_keywords() {
        for (src, tok) in [
            ("fn", Token::Fn), ("let", Token::Let), ("if", Token::If),
            ("else", Token::Else), ("loop", Token::Loop), ("return", Token::Return),
            ("struct", Token::Struct), ("enum", Token::Enum), ("impl", Token::Impl),
            ("pub", Token::Pub), ("use", Token::Use), ("async", Token::Async),
            ("await", Token::Await),
        ] {
            assert_eq!(toks(src), vec![tok, Token::Eof], "keyword {src}");
        }
    }

    #[test]
    fn identifiers_with_digits_underscores_and_hyphens() {
        assert_eq!(toks("my_var2"), vec![Token::Ident("my_var2".into()), Token::Eof]);
        assert_eq!(toks("kebab-case"), vec![Token::Ident("kebab-case".into()), Token::Eof]);
        assert_eq!(toks("_leading"), vec![Token::Ident("_leading".into()), Token::Eof]);
    }

    #[test]
    fn integer_and_underscored_integer() {
        assert_eq!(toks("42"), vec![Token::Int(42), Token::Eof]);
        assert_eq!(toks("1_000"), vec![Token::Int(1000), Token::Eof]);
    }

    #[test]
    fn float_requires_digit_after_dot() {
        assert_eq!(toks("3.14"), vec![Token::Float("3.14".into()), Token::Eof]);
        // `1.` with no following digit is Int(1) then Dot, not a float
        assert_eq!(toks("1."), vec![Token::Int(1), Token::Dot, Token::Eof]);
    }

    #[test]
    fn basic_string() {
        assert_eq!(toks("\"hello\""), vec![Token::Str("hello".into()), Token::Eof]);
    }

    #[test]
    fn string_with_escaped_quote_is_not_terminated_early() {
        // the backslash-escape skip means the embedded `\"` does not close the string
        assert_eq!(toks("\"a\\\"b\""), vec![Token::Str("a\\\"b".into()), Token::Eof]);
    }

    #[test]
    fn unterminated_string_reaches_eof_without_panicking() {
        assert_eq!(toks("\"never closed"), vec![Token::Str("never closed".into()), Token::Eof]);
    }

    #[test]
    fn line_comment_is_skipped_and_newline_survives() {
        // before the fix, the fallback `pos += 1` after a comment silently ate the
        // newline that follows it, so no Newline token was ever produced here
        assert_eq!(toks("::a comment\nfn"), vec![Token::Newline, Token::Fn, Token::Eof]);
    }

    #[test]
    fn line_comment_at_eof_with_no_trailing_newline() {
        assert_eq!(toks("::a comment"), vec![Token::Eof]);
    }

    #[test]
    fn block_comment_does_not_swallow_the_next_token() {
        // before the fix, the fallback `pos += 1` after a block comment ate the
        // first character that followed it: "/* c */fn" tokenized as Ident("n")
        assert_eq!(toks("/* c */fn"), vec![Token::Fn, Token::Eof]);
    }

    #[test]
    fn unterminated_block_comment_reaches_eof_without_panicking() {
        assert_eq!(toks("/* never closed"), vec![Token::Eof]);
    }

    #[test]
    fn unknown_ascii_char_is_skipped() {
        // backtick is not assigned to any token
        assert_eq!(toks("`fn"), vec![Token::Fn, Token::Eof]);
    }

    #[test]
    fn unknown_multibyte_char_is_skipped_without_panicking() {
        // before the fix, the fallback `pos += 1` advanced by one byte regardless
        // of the unknown character's UTF-8 width, landing mid-character on a
        // multi-byte char and panicking on the next slice
        assert_eq!(toks("中fn"), vec![Token::Fn, Token::Eof]);
        assert_eq!(toks("🎉fn"), vec![Token::Fn, Token::Eof]);
    }

    #[test]
    fn multibyte_identifier_content_in_string_is_preserved() {
        assert_eq!(toks("\"héllo 中\""), vec![Token::Str("héllo 中".into()), Token::Eof]);
    }

    #[test]
    fn span_covers_the_token_exactly() {
        let spanned = Lexer::new("  fn").tokenize();
        assert_eq!(spanned[0].span.start, 2);
        assert_eq!(spanned[0].span.end, 4);
    }
}
