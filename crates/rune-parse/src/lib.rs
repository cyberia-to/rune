/// Parser for rune's classic (`rust`) register.
/// Takes a token stream and produces rune_ast::Expr trees.
/// Pure register parser (`rune` sigil syntax) lands in M4.
use rune_ast::{Address, Expr};
use rune_lex::{Lexer, Span, Spanned, Token};

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

struct Parser {
    tokens: Vec<Spanned<Token>>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Spanned<Token>>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn parse_program(&mut self) -> Result<Expr, ParseError> {
        self.skip_newlines();
        let expr = self.parse_expr()?;
        self.skip_newlines();
        Ok(expr)
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.skip_newlines();
        match self.peek() {
            Token::Fn     => self.parse_fn(),
            Token::Let    => self.parse_let(),
            Token::If     => self.parse_if(),
            Token::Loop   => self.parse_loop(),
            Token::Return => self.parse_return(),
            Token::LBrack => self.parse_cell_literal(),
            Token::Hash   => self.parse_address_particle(),
            Token::Pat    => self.parse_address_neuron(),
            Token::Sig    => self.parse_address_name(),
            Token::Ket    => self.parse_address_abstract(),
            _             => self.parse_call_or_atom(),
        }
    }

    fn parse_fn(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::Fn)?;
        // optional name (discarded — gates are anonymous values)
        if matches!(self.peek(), Token::Ident(_)) { self.advance(); }
        self.expect(Token::LParen)?;
        let sample = self.parse_param_list()?;
        self.expect(Token::RParen)?;
        // optional return type annotation after `->`
        if matches!(self.peek(), Token::Arrow) {
            self.advance();
            self.parse_mold_expr()?; // consumed, not yet used
        }
        let body = self.parse_block()?;
        Ok(Expr::Gate { sample: Box::new(sample), body: Box::new(body) })
    }

    fn parse_param_list(&mut self) -> Result<Expr, ParseError> {
        // single param for now: `name: mold`
        if matches!(self.peek(), Token::RParen) {
            return Ok(Expr::Atom(0)); // no-sample gate
        }
        if let Token::Ident(name) = self.peek().clone() {
            self.advance();
            let _mold = if matches!(self.peek(), Token::Col) {
                self.advance();
                Some(Box::new(self.parse_mold_expr()?))
            } else { None };
            Ok(Expr::Sym(format!("param:{name}")))
        } else {
            Err(self.err("expected parameter name"))
        }
    }

    fn parse_mold_expr(&mut self) -> Result<Expr, ParseError> {
        // mold: @mold, #, or ident
        match self.peek().clone() {
            Token::Pat => {
                self.advance();
                if let Token::Ident(name) = self.peek().clone() {
                    self.advance();
                    Ok(Expr::Address(Address::Neuron(name)))
                } else { Ok(Expr::Address(Address::Neuron("ud".into()))) }
            }
            Token::Hash => {
                self.advance();
                Ok(Expr::Address(Address::Particle("*".into())))
            }
            Token::Ident(name) => { self.advance(); Ok(Expr::Sym(name)) }
            _ => Ok(Expr::Atom(0)),
        }
    }

    fn parse_let(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::Let)?;
        let name = self.expect_ident()?;
        let mold = if matches!(self.peek(), Token::Col) {
            self.advance();
            Some(Box::new(self.parse_mold_expr()?))
        } else { None };
        self.expect(Token::Tis)?;  // =
        let value = self.parse_expr()?;
        self.skip_semi_or_newline();
        let body = self.parse_expr()?;
        Ok(Expr::Let { name, mold, value: Box::new(value), body: Box::new(body) })
    }

    fn parse_if(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::If)?;
        let cond = self.parse_expr()?;
        let yes = self.parse_block()?;
        let no = if matches!(self.peek(), Token::Else) {
            self.advance();
            if matches!(self.peek(), Token::If) {
                self.parse_if()?
            } else {
                self.parse_block()?
            }
        } else { Expr::Atom(0) };
        Ok(Expr::If { cond: Box::new(cond), yes: Box::new(yes), no: Box::new(no) })
    }

    fn parse_loop(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::Loop)?;
        let body = self.parse_block()?;
        Ok(Expr::Trap(Box::new(body)))
    }

    fn parse_return(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::Return)?;
        self.parse_expr()
    }

    fn parse_block(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::LBrace)?;
        self.skip_newlines();
        let mut stmts = Vec::new();
        while !matches!(self.peek(), Token::RBrace | Token::Eof) {
            stmts.push(self.parse_expr()?);
            self.skip_semi_or_newline();
            self.skip_newlines();
        }
        self.expect(Token::RBrace)?;
        Ok(match stmts.len() {
            0 => Expr::Atom(0),
            1 => stmts.remove(0),
            _ => stmts.into_iter().rev().reduce(|body, val| {
                Expr::Let { name: "_".into(), mold: None, value: Box::new(val), body: Box::new(body) }
            }).unwrap(),
        })
    }

    fn parse_cell_literal(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::LBrack)?;
        let head = self.parse_expr()?;
        self.skip_comma_or_space();
        let tail = self.parse_expr()?;
        self.expect(Token::RBrack)?;
        Ok(Expr::Cell(Box::new(head), Box::new(tail)))
    }

    fn parse_call_or_atom(&mut self) -> Result<Expr, ParseError> {
        let base = self.parse_primary()?;
        // call: `gate(arg, arg)`
        if matches!(self.peek(), Token::LParen) {
            self.advance();
            let mut args = Vec::new();
            while !matches!(self.peek(), Token::RParen | Token::Eof) {
                args.push(self.parse_expr()?);
                if matches!(self.peek(), Token::Comma) { self.advance(); }
            }
            self.expect(Token::RParen)?;
            Ok(Expr::Call { gate: Box::new(base), args })
        } else {
            Ok(base)
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match self.peek().clone() {
            Token::Int(n)      => { self.advance(); Ok(Expr::Atom(n)) }
            Token::Ident(name) => { self.advance(); Ok(Expr::Sym(name)) }
            Token::LParen      => {
                self.advance();
                let inner = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(inner)
            }
            _ => Err(self.err(&format!("unexpected token: {:?}", self.peek()))),
        }
    }

    // Cybermark address forms
    fn parse_address_particle(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::Hash)?;
        let path = self.collect_path();
        Ok(Expr::Address(Address::Particle(path)))
    }
    fn parse_address_neuron(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::Pat)?;
        let name = self.collect_name();
        Ok(Expr::Address(Address::Neuron(name)))
    }
    fn parse_address_name(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::Sig)?;
        if matches!(self.peek(), Token::Fas) {
            self.advance();
            return Ok(Expr::Address(Address::Home));
        }
        let name = self.collect_name();
        Ok(Expr::Address(Address::Name(name)))
    }
    fn parse_address_abstract(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::Ket)?;
        let name = self.collect_name();
        Ok(Expr::Address(Address::Abstract(name)))
    }

    fn collect_path(&mut self) -> String {
        let mut parts = Vec::new();
        if let Token::Ident(s) = self.peek().clone() { self.advance(); parts.push(s); }
        while matches!(self.peek(), Token::Fas) {
            self.advance();
            if let Token::Ident(s) = self.peek().clone() { self.advance(); parts.push(s); }
        }
        parts.join("/")
    }

    fn collect_name(&mut self) -> String {
        if let Token::Ident(s) = self.peek().clone() { self.advance(); s }
        else { String::new() }
    }

    fn expect(&mut self, expected: Token) -> Result<(), ParseError> {
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(self.err(&format!("expected {expected:?}, got {:?}", self.peek())))
        }
    }

    fn expect_ident(&mut self) -> Result<String, ParseError> {
        if let Token::Ident(s) = self.peek().clone() { self.advance(); Ok(s) }
        else { Err(self.err("expected identifier")) }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek(), Token::Newline) { self.advance(); }
    }
    fn skip_semi_or_newline(&mut self) {
        while matches!(self.peek(), Token::Semi | Token::Newline) { self.advance(); }
    }
    fn skip_comma_or_space(&mut self) {
        if matches!(self.peek(), Token::Comma) { self.advance(); }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).map(|s| &s.inner).unwrap_or(&Token::Eof)
    }
    fn advance(&mut self) { if self.pos < self.tokens.len() { self.pos += 1; } }

    fn err(&self, msg: &str) -> ParseError {
        let span = self.tokens.get(self.pos)
            .map(|s| s.span.clone())
            .unwrap_or(rune_lex::Span { start: 0, end: 0 });
        ParseError { message: msg.to_string(), span }
    }
}

pub fn parse(src: &str) -> Result<Expr, ParseError> {
    let tokens = Lexer::new(src).tokenize();
    Parser::new(tokens).parse_program()
}
