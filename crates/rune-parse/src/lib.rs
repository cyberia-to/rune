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
            Token::LBrace => self.parse_block(),
            Token::LBrack => self.parse_cell_literal(),
            Token::Hash   => self.parse_address_particle(),
            Token::Pat    => self.parse_address_neuron(),
            Token::Sig    => self.parse_address_name(),
            Token::Ket    => self.parse_address_abstract(),
            _             => self.parse_binop(0),
        }
    }

    /// Pratt-style binary operator parsing.
    /// Precedence levels: 1=equality (== != < > <= >=), 2=additive (+ -), 3=multiplicative (*)
    fn parse_binop(&mut self, min_prec: u8) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_call_or_atom()?;

        loop {
            let (op, prec) = match self.peek().clone() {
                Token::Lus  => ("add", 2),
                Token::Ident(ref s) if s == "-" => ("sub", 2),
                Token::Star => ("mul", 3),
                Token::Tis  => {
                    // peek at next token to decide: == (equality) or standalone =
                    if matches!(self.peek_at(1), Token::Tis) {
                        ("eq", 1)
                    } else {
                        break;
                    }
                }
                Token::Zap => {
                    // != — Zap then Tis
                    if matches!(self.peek_at(1), Token::Tis) {
                        ("neq", 1)
                    } else {
                        break;
                    }
                }
                Token::Wut => {
                    // potential < or >; handled via Ident for < and >
                    break;
                }
                Token::Ident(ref s) if s == "<" => ("lt", 1),
                Token::Ident(ref s) if s == ">" => ("gt", 1),
                _ => break,
            };

            if prec < min_prec { break; }

            // consume operator token(s)
            match op {
                "eq" | "neq" => { self.advance(); self.advance(); } // consume two tokens
                _ => { self.advance(); }
            }

            let rhs = self.parse_call_or_atom()?;
            lhs = match op {
                "add" => Expr::Call { gate: Box::new(Expr::Sym("add".into())), args: vec![lhs, rhs] },
                "sub" => Expr::Call { gate: Box::new(Expr::Sym("sub".into())), args: vec![lhs, rhs] },
                "mul" => Expr::Call { gate: Box::new(Expr::Sym("mul".into())), args: vec![lhs, rhs] },
                "eq"  => Expr::Eq(Box::new(lhs), Box::new(rhs)),
                "neq" => Expr::Call {
                    gate: Box::new(Expr::Sym("not".into())),
                    args: vec![Expr::Eq(Box::new(lhs), Box::new(rhs))],
                },
                "lt"  => Expr::Call { gate: Box::new(Expr::Sym("lt".into())), args: vec![lhs, rhs] },
                "gt"  => Expr::Call { gate: Box::new(Expr::Sym("gt".into())), args: vec![lhs, rhs] },
                _ => unreachable!(),
            };
        }

        Ok(lhs)
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
        if matches!(self.peek(), Token::RParen) {
            return Ok(Expr::Atom(0)); // no-sample gate
        }

        // Collect all params
        let mut params: Vec<String> = Vec::new();
        loop {
            if let Token::Ident(name) = self.peek().clone() {
                self.advance();
                // optional mold annotation: name: @mold
                if matches!(self.peek(), Token::Col) {
                    self.advance();
                    self.parse_mold_expr()?; // consumed, ignored for now
                }
                params.push(name);
            } else {
                return Err(self.err("expected parameter name"));
            }
            if matches!(self.peek(), Token::Comma) {
                self.advance();
                if matches!(self.peek(), Token::RParen) { break; }
            } else {
                break;
            }
        }

        match params.len() {
            0 => Ok(Expr::Atom(0)),
            1 => Ok(Expr::Sym(format!("param:{}", params[0]))),
            _ => {
                // multi-param: semicolon-joined name list as a single sym
                Ok(Expr::Sym(format!("param:{}", params.join(";"))))
            }
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

    /// Parse a `{ stmt; stmt; final_expr }` block.
    /// Multiple statements produce nested Let { name: "_", ... } bindings.
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
            _ => {
                // fold right: last element is the final body, earlier ones are sequenced
                let mut iter = stmts.into_iter().rev();
                let last = iter.next().unwrap();
                iter.fold(last, |body, val| {
                    Expr::Let { name: "_".into(), mold: None, value: Box::new(val), body: Box::new(body) }
                })
            }
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
            Token::Str(s)      => {
                self.advance();
                let atom = string_to_atom(&s);
                Ok(Expr::Atom(atom))
            }
            Token::Ident(name) => {
                self.advance();
                match name.as_str() {
                    "true"  => Ok(Expr::Atom(0)),
                    "false" => Ok(Expr::Atom(1)),
                    _       => Ok(Expr::Sym(name)),
                }
            }
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

    fn peek_at(&self, offset: usize) -> &Token {
        self.tokens.get(self.pos + offset).map(|s| &s.inner).unwrap_or(&Token::Eof)
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

/// Minimal name→atom encoding (matching rune-lower's stub).
fn string_to_atom(s: &str) -> u64 {
    s.bytes().fold(0u64, |a, b| a.wrapping_mul(31).wrapping_add(b as u64))
}

/// Pretty-print an Expr back to classic (rust-style) register syntax.
pub fn format_expr(expr: &rune_ast::Expr) -> String {
    use rune_ast::Expr;
    match expr {
        Expr::Atom(n) => n.to_string(),
        Expr::Sym(s) => s.clone(),
        Expr::Cell(h, t) => format!("[{}, {}]", format_expr(h), format_expr(t)),
        Expr::Let { name, value, body, .. } =>
            format!("let {} = {};\n{}", name, format_expr(value), format_expr(body)),
        Expr::If { cond, yes, no } =>
            format!("if {} {{ {} }} else {{ {} }}", format_expr(cond), format_expr(yes), format_expr(no)),
        Expr::Gate { sample, body } =>
            format!("fn({}) {{ {} }}", format_expr(sample), format_expr(body)),
        Expr::Call { gate, args } =>
            format!("{}({})", format_expr(gate), args.iter().map(format_expr).collect::<Vec<_>>().join(", ")),
        Expr::Eq(a, b) => format!("{} == {}", format_expr(a), format_expr(b)),
        Expr::Inc(x) => format!("{} + 1", format_expr(x)),
        Expr::Trap(body) => format!("loop {{ {} }}", format_expr(body)),
        Expr::Address(a) => format!("{a:?}"),
        _ => format!("{expr:?}"),
    }
}

pub enum Register { Classic }

pub fn parse_with_register(src: &str, _reg: Register) -> Result<rune_ast::Expr, ParseError> {
    parse(src)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_atom() {
        let e = parse("42").unwrap();
        assert!(matches!(e, Expr::Atom(42)));
    }

    #[test]
    fn parse_add_binop() {
        let e = parse("1 + 2").unwrap();
        match e {
            Expr::Call { gate, args } => {
                assert!(matches!(*gate, Expr::Sym(ref s) if s == "add"));
                assert_eq!(args.len(), 2);
            }
            _ => panic!("expected Call"),
        }
    }

    #[test]
    fn parse_eq_binop() {
        let e = parse("x == 0").unwrap();
        assert!(matches!(e, Expr::Eq(_, _)));
    }

    #[test]
    fn parse_string_literal() {
        let e = parse("\"hello\"").unwrap();
        assert!(matches!(e, Expr::Atom(_)));
    }

    #[test]
    fn parse_bool_true() {
        let e = parse("true").unwrap();
        assert!(matches!(e, Expr::Atom(0)));
    }

    #[test]
    fn parse_bool_false() {
        let e = parse("false").unwrap();
        assert!(matches!(e, Expr::Atom(1)));
    }

    #[test]
    fn parse_block_sequencing() {
        let e = parse("{ 1; 2; 3 }").unwrap();
        // Should be Let { "_", 1, Let { "_", 2, 3 } }
        match e {
            Expr::Let { name, value, body, .. } => {
                assert_eq!(name, "_");
                assert!(matches!(*value, Expr::Atom(1)));
                match *body {
                    Expr::Let { name: ref n2, value: ref v2, body: ref b2, .. } => {
                        assert_eq!(n2, "_");
                        assert!(matches!(**v2, Expr::Atom(2)));
                        assert!(matches!(**b2, Expr::Atom(3)));
                    }
                    _ => panic!("expected nested Let"),
                }
            }
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn parse_multi_param() {
        let e = parse("fn(x, y) { x }").unwrap();
        match e {
            Expr::Gate { sample, .. } => {
                assert!(matches!(*sample, Expr::Sym(ref s) if s == "param:x;y"));
            }
            _ => panic!("expected Gate"),
        }
    }
}
