/// Parser for rune's pure (rune) register — sigil/digraph syntax (Hoon-like).
///
/// The pure register uses digraph runes instead of infix/keyword syntax.
/// Only flat (parenthesized) form is supported here.
///
/// Entry points:
///   `parse(src) -> Result<Expr, ParseError>` — parse a pure-register expression
///   `format_expr(expr) -> String`            — pretty-print in pure-register style
use rune_ast::{Address, Expr};
use rune_lex::{Lexer, Span, Spanned, Token};
use cyber_hemera as hemera;

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Digraph table
// ---------------------------------------------------------------------------

/// All 25+ digraphs recognised by the pure register.
#[derive(Debug, Clone, PartialEq)]
enum Digraph {
    /// `|=` — gate (door sample + body)
    BarTis,
    /// `|*` — wet gate (same AST as gate for now)
    BarStar,
    /// `|-` — trap (loop)
    BarHep,
    /// `|%` — door (multi-arm core)
    BarCen,
    /// `=/` — let binding (name, value, body)
    TisFas,
    /// `=.` — rebind slot
    TisDot,
    /// `=>` — compose
    TisFatArrow,
    /// `?:` — if-then-else
    WutCol,
    /// `?-` — match/switch
    WutHep,
    /// `?=` — pattern-match if (simplified: same as if)
    WutTis,
    /// `?~` — if-null branch
    WutSig,
    /// `:-` — cell pair
    ColHep,
    /// `:+` — three-cell (a b c) → [a [b c]]
    ColLus,
    /// `:_` — flipped pair [tail head]
    ColCab,
    /// `^-` — cast (mold expr)
    KetHep,
    /// `^+` — cast-like (mold-expr expr)
    KetLus,
    /// `.*` — eval (subject formula)
    DotStar,
    /// `.+` — increment
    DotLus,
    /// `.^` — scry
    DotKet,
    /// `~&` — trace hint
    SigAmp,
    /// `~_` — hint-pair
    SigCab,
    /// `/+` — import lib
    FasLus,
    /// `/-` — import types
    FasHep,
    /// `++` — arm definition
    LusLus,
    /// `!!` — crash
    ZapZap,
}

// ---------------------------------------------------------------------------
// Lexer wrapper — we reuse rune_lex but need to recognise digraphs
// ---------------------------------------------------------------------------

struct Parser {
    tokens: Vec<Spanned<Token>>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Spanned<Token>>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn parse_program(&mut self) -> Result<Expr, ParseError> {
        self.skip_ws();
        let expr = self.parse_expr()?;
        self.skip_ws();
        Ok(expr)
    }

    // -----------------------------------------------------------------------
    // Expression dispatch
    // -----------------------------------------------------------------------

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.skip_ws();
        match self.peek().clone() {
            Token::LParen  => self.parse_paren_form(),
            Token::LBrack  => self.parse_bracket_cell(),
            Token::Int(n)  => { self.advance(); Ok(Expr::Atom(n)) }
            Token::Str(s)  => { self.advance(); Ok(Expr::Text(s)) }
            Token::Percent => {
                // %term — term literal
                self.advance();
                if let Token::Ident(name) = self.peek().clone() {
                    self.advance();
                    Ok(Expr::Atom(string_to_atom(&name)))
                } else {
                    Ok(Expr::Atom(0))
                }
            }
            Token::Ident(name) => {
                self.advance();
                match name.as_str() {
                    "true"  => Ok(Expr::Atom(0)),
                    "false" => Ok(Expr::Atom(1)),
                    _       => Ok(Expr::Sym(name)),
                }
            }
            Token::Zap => {
                // `!!` — crash (two consecutive `!`)
                if matches!(self.peek_at(1), Token::Zap) {
                    self.advance(); self.advance();
                    return Ok(Expr::Call { gate: Box::new(Expr::Sym("crash".into())), args: vec![] });
                }
                Err(self.err(&format!("unexpected token: {:?}", self.peek())))
            }
            Token::Lus => {
                // `++` — arm (two consecutive `+`)
                if matches!(self.peek_at(1), Token::Lus) {
                    return self.parse_arm_bare();
                }
                Err(self.err("unexpected `+`"))
            }
            _ => Err(self.err(&format!("unexpected token: {:?}", self.peek()))),
        }
    }

    // `[ expr expr ]` — cell literal
    fn parse_bracket_cell(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::LBrack)?;
        self.skip_ws();
        let head = self.parse_expr()?;
        self.skip_ws();
        let tail = self.parse_expr()?;
        self.skip_ws();
        self.expect(Token::RBrack)?;
        Ok(Expr::Cell(Box::new(head), Box::new(tail)))
    }

    // `( digraph expr... )` — flat rune form
    fn parse_paren_form(&mut self) -> Result<Expr, ParseError> {
        self.expect(Token::LParen)?;
        self.skip_ws();

        // Attempt to read a digraph
        let dg = self.try_parse_digraph();
        if let Some(dg) = dg {
            let result = self.parse_digraph_body(dg)?;
            self.skip_ws();
            self.expect(Token::RParen)?;
            Ok(result)
        } else {
            // Not a digraph — treat as a call: (gate arg arg ...)
            let gate = self.parse_expr()?;
            self.skip_ws();
            let mut args = Vec::new();
            while !matches!(self.peek(), Token::RParen | Token::Eof) {
                args.push(self.parse_expr()?);
                self.skip_ws();
            }
            self.expect(Token::RParen)?;
            Ok(Expr::Call { gate: Box::new(gate), args })
        }
    }

    // Try to recognise a two-token digraph at current position.
    // Returns Some(dg) and advances past both tokens, or None with no advance.
    fn try_parse_digraph(&mut self) -> Option<Digraph> {
        let t0 = self.peek().clone();
        let t1 = self.peek_at(1).clone();

        let dg = match (&t0, &t1) {
            (Token::Bar,     Token::Tis)      => Some(Digraph::BarTis),
            (Token::Bar,     Token::Star)     => Some(Digraph::BarStar),
            (Token::Ident(s), _) if s == "-"  => None, // hep is lexed as Ident("-")
            (Token::Bar,     Token::Percent)  => Some(Digraph::BarCen),
            (Token::Tis,     Token::Fas)      => Some(Digraph::TisFas),
            (Token::Tis,     Token::Dot)      => Some(Digraph::TisDot),
            (Token::Tis,     Token::FatArrow) => Some(Digraph::TisFatArrow),
            (Token::Wut,     Token::Col)      => Some(Digraph::WutCol),
            (Token::Wut,     Token::Tis)      => Some(Digraph::WutTis),
            (Token::Wut,     Token::Sig)      => Some(Digraph::WutSig),
            (Token::Col,     Token::Lus)      => Some(Digraph::ColLus),
            (Token::Ket,     Token::Arrow)    => Some(Digraph::KetHep),  // ^-
            (Token::Ket,     Token::Lus)      => Some(Digraph::KetLus),
            (Token::Dot,     Token::Star)     => Some(Digraph::DotStar),
            (Token::Dot,     Token::Lus)      => Some(Digraph::DotLus),
            (Token::Dot,     Token::Ket)      => Some(Digraph::DotKet),
            (Token::Sig,     Token::Amp)      => Some(Digraph::SigAmp),
            (Token::Fas,     Token::Lus)      => Some(Digraph::FasLus),
            (Token::Lus,     Token::Lus)      => Some(Digraph::LusLus),
            (Token::Zap,     Token::Zap)      => Some(Digraph::ZapZap),
            _ => None,
        };

        // Handle hep-based digraphs (lexed differently):
        // `|-` → Bar then Ident("-"); `:-` → Col then Ident("-")
        // `?-` → Wut then Ident("-"); `/-` → Fas then Ident("-")
        // `~_` → Sig then … (underscore lexes as Ident("_"))
        let dg = dg.or_else(|| match (&t0, &t1) {
            (Token::Bar, Token::Ident(s)) if s == "-" => Some(Digraph::BarHep),
            (Token::Col, Token::Ident(s)) if s == "-" => Some(Digraph::ColHep),
            (Token::Wut, Token::Ident(s)) if s == "-" => Some(Digraph::WutHep),
            (Token::Fas, Token::Ident(s)) if s == "-" => Some(Digraph::FasHep),
            (Token::Sig, Token::Ident(s)) if s == "_" => Some(Digraph::SigCab),
            (Token::Col, Token::Ident(s)) if s == "_" => Some(Digraph::ColCab),
            _ => None,
        });

        if let Some(dg) = dg {
            self.advance(); // consume first char token
            self.advance(); // consume second char token
            Some(dg)
        } else {
            None
        }
    }

    fn parse_digraph_body(&mut self, dg: Digraph) -> Result<Expr, ParseError> {
        self.skip_ws();
        match dg {
            // |= sample body  — gate
            Digraph::BarTis | Digraph::BarStar => {
                let sample = self.parse_sample()?;
                self.skip_ws();
                let body = self.parse_expr()?;
                Ok(Expr::Gate { sample: Box::new(sample), body: Box::new(body) })
            }

            // |- body  — trap
            Digraph::BarHep => {
                let body = self.parse_expr()?;
                Ok(Expr::Trap(Box::new(body)))
            }

            // |% arm...  — door (multi-arm core)
            Digraph::BarCen => {
                let mut arms = Vec::new();
                while !matches!(self.peek(), Token::RParen | Token::Eof) {
                    self.skip_ws();
                    arms.push(self.parse_expr()?);
                    self.skip_ws();
                }
                Ok(Expr::Door(arms))
            }

            // =/ name value body  — let
            Digraph::TisFas => {
                let name = self.parse_name_or_typed_name()?;
                self.skip_ws();
                let value = self.parse_expr()?;
                self.skip_ws();
                let body = self.parse_expr()?;
                Ok(Expr::Let { name, mold: None, value: Box::new(value), body: Box::new(body) })
            }

            // =. slot value body  — rebind
            Digraph::TisDot => {
                let slot = self.parse_name_or_typed_name()?;
                self.skip_ws();
                let value = self.parse_expr()?;
                self.skip_ws();
                let body = self.parse_expr()?;
                Ok(Expr::Rebind { slot, value: Box::new(value), body: Box::new(body) })
            }

            // => left right  — compose
            Digraph::TisFatArrow => {
                let left = self.parse_expr()?;
                self.skip_ws();
                let right = self.parse_expr()?;
                Ok(Expr::Compose(Box::new(left), Box::new(right)))
            }

            // ?: cond yes no  — if
            Digraph::WutCol => {
                let cond = self.parse_expr()?;
                self.skip_ws();
                let yes = self.parse_expr()?;
                self.skip_ws();
                let no = self.parse_expr()?;
                Ok(Expr::If { cond: Box::new(cond), yes: Box::new(yes), no: Box::new(no) })
            }

            // ?- subject arm...  — match/switch
            Digraph::WutHep => {
                let subject = self.parse_expr()?;
                self.skip_ws();
                let mut arms = Vec::new();
                while !matches!(self.peek(), Token::RParen | Token::Eof) {
                    let pat = self.parse_expr()?;
                    self.skip_ws();
                    let body = self.parse_expr()?;
                    self.skip_ws();
                    arms.push((pat, body));
                }
                Ok(Expr::Match { subject: Box::new(subject), arms })
            }

            // ?= (simplified: pattern-match if, same as If)
            Digraph::WutTis => {
                let cond = self.parse_expr()?;
                self.skip_ws();
                let yes = self.parse_expr()?;
                self.skip_ws();
                let no = self.parse_expr()?;
                Ok(Expr::If { cond: Box::new(cond), yes: Box::new(yes), no: Box::new(no) })
            }

            // ?~ expr yes no  — if-null: if expr == 0 then yes else no
            Digraph::WutSig => {
                let expr = self.parse_expr()?;
                self.skip_ws();
                let yes = self.parse_expr()?;
                self.skip_ws();
                let no = self.parse_expr()?;
                Ok(Expr::If {
                    cond: Box::new(Expr::Eq(Box::new(expr), Box::new(Expr::Atom(0)))),
                    yes: Box::new(yes),
                    no: Box::new(no),
                })
            }

            // :- head tail  — cell pair
            Digraph::ColHep => {
                let head = self.parse_expr()?;
                self.skip_ws();
                let tail = self.parse_expr()?;
                Ok(Expr::Cell(Box::new(head), Box::new(tail)))
            }

            // :+ a b c  → [a [b c]]
            Digraph::ColLus => {
                let a = self.parse_expr()?;
                self.skip_ws();
                let b = self.parse_expr()?;
                self.skip_ws();
                let c = self.parse_expr()?;
                Ok(Expr::Cell(Box::new(a), Box::new(Expr::Cell(Box::new(b), Box::new(c)))))
            }

            // :_ tail head  → [head tail] (flipped)
            Digraph::ColCab => {
                let tail = self.parse_expr()?;
                self.skip_ws();
                let head = self.parse_expr()?;
                Ok(Expr::Cell(Box::new(head), Box::new(tail)))
            }

            // ^- mold expr  — cast
            Digraph::KetHep => {
                let mold = self.parse_expr()?;
                self.skip_ws();
                let expr = self.parse_expr()?;
                Ok(Expr::Cast { mold: Box::new(mold), expr: Box::new(expr) })
            }

            // ^+ mold-expr expr
            Digraph::KetLus => {
                let mold = self.parse_expr()?;
                self.skip_ws();
                let expr = self.parse_expr()?;
                Ok(Expr::Cast { mold: Box::new(mold), expr: Box::new(expr) })
            }

            // .* subject formula  — eval
            Digraph::DotStar => {
                let subject = self.parse_expr()?;
                self.skip_ws();
                let formula = self.parse_expr()?;
                Ok(Expr::Eval { subject: Box::new(subject), formula: Box::new(formula) })
            }

            // .+ expr  — increment
            Digraph::DotLus => {
                let expr = self.parse_expr()?;
                Ok(Expr::Inc(Box::new(expr)))
            }

            // .^ mold path  — scry
            Digraph::DotKet => {
                let mold = self.parse_expr()?;
                self.skip_ws();
                let path = self.parse_expr()?;
                Ok(Expr::Scry { mold: Box::new(mold), path: Box::new(path) })
            }

            // ~& selector body  — trace hint
            Digraph::SigAmp => {
                let selector = self.parse_expr()?;
                self.skip_ws();
                let body = self.parse_expr()?;
                Ok(Expr::Hint { tag: "trace".into(), selector: Box::new(selector), body: Box::new(body) })
            }

            // ~_ selector body  — hint-pair
            Digraph::SigCab => {
                let selector = self.parse_expr()?;
                self.skip_ws();
                let body = self.parse_expr()?;
                Ok(Expr::Hint { tag: "hint-pair".into(), selector: Box::new(selector), body: Box::new(body) })
            }

            // /+ name  — import lib
            Digraph::FasLus => {
                let name = self.parse_expr()?;
                Ok(Expr::Call { gate: Box::new(Expr::Sym("import-lib".into())), args: vec![name] })
            }

            // /- name  — import types
            Digraph::FasHep => {
                let name = self.parse_expr()?;
                Ok(Expr::Call { gate: Box::new(Expr::Sym("import-types".into())), args: vec![name] })
            }

            // ++ name body  — arm definition
            Digraph::LusLus => {
                let name = self.parse_name_or_typed_name()?;
                self.skip_ws();
                let body = self.parse_expr()?;
                Ok(Expr::Arm { name, body: Box::new(body) })
            }

            // !! — crash
            Digraph::ZapZap => {
                Ok(Expr::Call { gate: Box::new(Expr::Sym("crash".into())), args: vec![] })
            }
        }
    }

    // Parse a bare `++ name body` without the enclosing parens (used inside |% bodies)
    fn parse_arm_bare(&mut self) -> Result<Expr, ParseError> {
        // consume ++
        self.advance(); self.advance();
        self.skip_ws();
        let name = self.parse_name_or_typed_name()?;
        self.skip_ws();
        let body = self.parse_expr()?;
        Ok(Expr::Arm { name, body: Box::new(body) })
    }

    // -----------------------------------------------------------------------
    // Sample / name parsing
    // -----------------------------------------------------------------------

    /// Parse a sample specification: either `name=@mold` or a bare `name`.
    /// Returns the sample Expr (Sym for a named param, Atom(0) for no-op).
    fn parse_sample(&mut self) -> Result<Expr, ParseError> {
        match self.peek().clone() {
            Token::Ident(name) => {
                self.advance();
                // name=@mold ?
                if matches!(self.peek(), Token::Tis) && !matches!(self.peek_at(1), Token::Tis) {
                    self.advance(); // consume =
                    self.parse_mold_expr()?; // consume mold, ignored for now
                }
                Ok(Expr::Sym(format!("param:{name}")))
            }
            Token::LBrack => {
                // [a b] — multi-sample via cell
                self.advance();
                self.skip_ws();
                let mut params = Vec::new();
                while !matches!(self.peek(), Token::RBrack | Token::Eof) {
                    if let Token::Ident(n) = self.peek().clone() {
                        self.advance();
                        params.push(n);
                        if matches!(self.peek(), Token::Tis) {
                            self.advance();
                            self.parse_mold_expr()?;
                        }
                    } else {
                        break;
                    }
                    self.skip_ws();
                }
                self.expect(Token::RBrack)?;
                match params.len() {
                    0 => Ok(Expr::Atom(0)),
                    1 => Ok(Expr::Sym(format!("param:{}", params[0]))),
                    _ => Ok(Expr::Sym(format!("param:{}", params.join(";")))),
                }
            }
            Token::LParen => {
                // Could be nested expression used as sample
                self.parse_expr()
            }
            _ => Ok(Expr::Atom(0)), // no sample
        }
    }

    /// Parse a name, possibly with `=@mold` annotation. Returns the plain name string.
    fn parse_name_or_typed_name(&mut self) -> Result<String, ParseError> {
        if let Token::Ident(name) = self.peek().clone() {
            self.advance();
            // optional =@mold annotation (name=@mold)
            if matches!(self.peek(), Token::Tis) && !matches!(self.peek_at(1), Token::Tis) {
                self.advance(); // consume =
                self.parse_mold_expr()?; // ignore mold for now
            }
            Ok(name)
        } else {
            Err(self.err("expected name"))
        }
    }

    fn parse_mold_expr(&mut self) -> Result<Expr, ParseError> {
        match self.peek().clone() {
            Token::Pat => {
                self.advance();
                if let Token::Ident(name) = self.peek().clone() {
                    self.advance();
                    Ok(Expr::Address(Address::Neuron(name)))
                } else {
                    Ok(Expr::Address(Address::Neuron("ud".into())))
                }
            }
            Token::Hash => {
                self.advance();
                Ok(Expr::Address(Address::Particle("*".into())))
            }
            Token::Ident(name) => { self.advance(); Ok(Expr::Sym(name)) }
            Token::LParen => self.parse_paren_form(),
            _ => Ok(Expr::Atom(0)),
        }
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Token::Newline | Token::Indent | Token::Dedent) {
            self.advance();
        }
        // Also skip horizontal whitespace tokens if any (Lexer skips most already)
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).map(|s| &s.inner).unwrap_or(&Token::Eof)
    }

    fn peek_at(&self, offset: usize) -> &Token {
        self.tokens.get(self.pos + offset).map(|s| &s.inner).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() { self.pos += 1; }
    }

    fn expect(&mut self, expected: Token) -> Result<(), ParseError> {
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(self.err(&format!("expected {:?}, got {:?}", expected, self.peek())))
        }
    }

    fn err(&self, msg: &str) -> ParseError {
        let span = self.tokens.get(self.pos)
            .map(|s| s.span.clone())
            .unwrap_or(Span { start: 0, end: 0 });
        ParseError { message: msg.to_string(), span }
    }
}

// ---------------------------------------------------------------------------
// Minimal name→atom encoding (matches rune-lower's placeholder)
// ---------------------------------------------------------------------------

fn string_to_atom(s: &str) -> u64 {
    let digest = hemera::hash(s.as_bytes());
    u64::from_le_bytes(digest.as_bytes()[..8].try_into().unwrap())
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn parse(src: &str) -> Result<Expr, ParseError> {
    let tokens = Lexer::new(src).tokenize();
    Parser::new(tokens).parse_program()
}

/// Pretty-print an Expr in pure register (sigil) syntax.
pub fn format_expr(expr: &Expr) -> String {
    match expr {
        Expr::Atom(n)  => n.to_string(),
        Expr::Text(s)  => format!("{s:?}"),
        Expr::Sym(s)   => s.clone(),
        Expr::Cell(h, t) => format!(":-  {}\n{}", format_expr(h), format_expr(t)),
        Expr::Let { name, value, body, .. } =>
            format!("=/  {}\n{}\n{}", name, format_expr(value), format_expr(body)),
        Expr::If { cond, yes, no } =>
            format!("?:  {}\n{}\n{}", format_expr(cond), format_expr(yes), format_expr(no)),
        Expr::Gate { sample, body } => {
            let sample_str = match sample.as_ref() {
                Expr::Sym(s) if s.starts_with("param:") => s["param:".len()..].replace(';', " "),
                Expr::Atom(0) => "~".into(),
                _ => format_expr(sample),
            };
            format!("|=  {}\n{}", sample_str, format_expr(body))
        }
        Expr::Call { gate: gate_expr, args } => {
            if let Expr::Sym(op) = gate_expr.as_ref() {
                match op.as_str() {
                    "add" | "sub" | "mul" | "lt" | "gt" => {
                        let joined = args.iter().map(format_expr).collect::<Vec<_>>().join(" ");
                        return format!("({op} {joined})");
                    }
                    "crash" => return "!!".into(),
                    "import-lib" => {
                        let n = args.first().map(format_expr).unwrap_or_default();
                        return format!("/+  {n}");
                    }
                    "import-types" => {
                        let n = args.first().map(format_expr).unwrap_or_default();
                        return format!("/-  {n}");
                    }
                    _ => {}
                }
            }
            let joined = args.iter().map(format_expr).collect::<Vec<_>>().join(" ");
            format!("({} {joined})", format_expr(gate_expr))
        }
        Expr::Eq(a, b) => format!("(=(eq) {} {})", format_expr(a), format_expr(b)),
        Expr::Inc(x)   => format!("(.+ {})", format_expr(x)),
        Expr::Trap(body) => format!("(|- {})", format_expr(body)),
        Expr::Eval { subject, formula } =>
            format!("(.* {} {})", format_expr(subject), format_expr(formula)),
        Expr::Scry { mold, path } =>
            format!("(.^ {} {})", format_expr(mold), format_expr(path)),
        Expr::Cast { mold, expr } =>
            format!("(^- {} {})", format_expr(mold), format_expr(expr)),
        Expr::Compose(l, r) =>
            format!("(=> {} {})", format_expr(l), format_expr(r)),
        Expr::Hint { tag, selector, body } =>
            format!("(~& {tag} {} {})", format_expr(selector), format_expr(body)),
        Expr::Arm { name, body } =>
            format!("++  {}\n{}", name, format_expr(body)),
        Expr::Door(arms) => {
            let body = arms.iter().map(format_expr).collect::<Vec<_>>().join("\n");
            format!("|%\n{body}\n--")
        }
        Expr::Rebind { slot, value, body } =>
            format!("(=. {} {} {})", slot, format_expr(value), format_expr(body)),
        Expr::Match { subject, arms } => {
            let arm_strs = arms.iter()
                .map(|(p, b)| format!("{} {}", format_expr(p), format_expr(b)))
                .collect::<Vec<_>>()
                .join("\n");
            format!("(?- {}\n{})", format_expr(subject), arm_strs)
        }
        Expr::Loop => "$".into(),
        Expr::MethodCall { obj, method, args } => {
            let joined = args.iter().map(format_expr).collect::<Vec<_>>().join(" ");
            format!("({obj}-{method} {joined})")
        }
        Expr::Address(a) => format!("{a:?}"),
        _ => format!("{expr:?}"),
    }
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
    fn parse_term() {
        // %truth → Atom(string_to_atom("truth"))
        let e = parse("%truth").unwrap();
        assert!(matches!(e, Expr::Atom(_)));
    }

    #[test]
    fn parse_gate() {
        // (|= x (add x 1))
        let e = parse("(|= x (add x 1))").unwrap();
        match e {
            Expr::Gate { sample, body } => {
                assert!(matches!(*sample, Expr::Sym(ref s) if s == "param:x"));
                match *body {
                    Expr::Call { ref gate, ref args } => {
                        assert!(matches!(*gate.as_ref(), Expr::Sym(ref s) if s == "add"));
                        assert_eq!(args.len(), 2);
                    }
                    _ => panic!("expected Call"),
                }
            }
            _ => panic!("expected Gate"),
        }
    }

    #[test]
    fn parse_if() {
        let e = parse("(?: 0 1 2)").unwrap();
        assert!(matches!(e, Expr::If { .. }));
    }

    #[test]
    fn parse_increment() {
        let e = parse("(.+ 5)").unwrap();
        assert!(matches!(e, Expr::Inc(_)));
    }

    #[test]
    fn parse_let() {
        let e = parse("(=/ x 1 x)").unwrap();
        match e {
            Expr::Let { name, value, body, .. } => {
                assert_eq!(name, "x");
                assert!(matches!(*value, Expr::Atom(1)));
                assert!(matches!(*body, Expr::Sym(ref s) if s == "x"));
            }
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn parse_cell_literal() {
        let e = parse("[1 2]").unwrap();
        assert!(matches!(e, Expr::Cell(_, _)));
    }

    #[test]
    fn parse_trap() {
        let e = parse("(|- 0)").unwrap();
        assert!(matches!(e, Expr::Trap(_)));
    }

    #[test]
    fn parse_crash() {
        let e = parse("!!").unwrap();
        match e {
            Expr::Call { gate, args } => {
                assert!(matches!(*gate, Expr::Sym(ref s) if s == "crash"));
                assert!(args.is_empty());
            }
            _ => panic!("expected crash call"),
        }
    }

    /// The M4 completion signal: the same program in both registers parses to
    /// the same AST. Compared via Debug since Expr has no PartialEq.
    #[test]
    fn double_is_the_same_ast_in_both_registers() {
        let classic = rune_parse::parse("fn double(x: @ud) { x * 2 }").unwrap();
        let pure = parse("(|= x=@ud (mul x 2))").unwrap();
        assert_eq!(format!("{classic:?}"), format!("{pure:?}"));
    }

    #[test]
    fn format_gate_roundtrip() {
        let e = Expr::Gate {
            sample: Box::new(Expr::Sym("param:x".into())),
            body: Box::new(Expr::Inc(Box::new(Expr::Sym("x".into())))),
        };
        let s = format_expr(&e);
        assert!(s.contains("|="));
    }
}
