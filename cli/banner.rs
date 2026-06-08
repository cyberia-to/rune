//! The rune banner and help text — matched to hemera's CLI aesthetic:
//! a gradient ASCII-art mark, a white tagline, a gray spec block, and an
//! aligned command list.

const R: &str = "\x1b[31m"; // red
const Y: &str = "\x1b[33m"; // yellow
const G: &str = "\x1b[32m"; // green
const C: &str = "\x1b[36m"; // cyan
const B: &str = "\x1b[34m"; // blue
const M: &str = "\x1b[35m"; // magenta
const W: &str = "\x1b[37m"; // white
const GR: &str = "\x1b[90m"; // gray
const X: &str = "\x1b[0m"; // reset

/// Colour `s` with `col` when `color`, else return it plain.
fn paint(color: bool, col: &str, s: &str) -> String {
    if color {
        format!("{col}{s}{X}")
    } else {
        s.to_string()
    }
}

/// The colored rune mark + tagline + spec block.
pub fn banner(color: bool) -> String {
    let line = |col: &str, s: &str| paint(color, col, s);
    let mut o = String::new();
    o.push('\n');
    o.push_str(&line(R, "    ██████╗ ██╗   ██╗███╗   ██╗███████╗\n"));
    o.push_str(&line(Y, "    ██╔══██╗██║   ██║████╗  ██║██╔════╝\n"));
    o.push_str(&line(G, "    ██████╔╝██║   ██║██╔██╗ ██║█████╗  \n"));
    o.push_str(&line(C, "    ██╔══██╗██║   ██║██║╚██╗██║██╔══╝  \n"));
    o.push_str(&line(B, "    ██║  ██║╚██████╔╝██║ ╚████║███████╗\n"));
    o.push_str(&line(M, "    ╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═══╝╚══════╝\n"));
    o.push_str(&line(W, "    open computation for the cybergraph\n"));
    o.push('\n');
    o.push_str(&line(GR, "    Nox · 18-pattern tree reduction · Goldilocks field\n"));
    o.push_str(&line(GR, "    two registers (classic + pure) · one AST · instant start\n"));
    o
}

/// The command list shown by `rune help`.
pub fn help(color: bool) -> String {
    let g = |s: &str| paint(color, GR, s);
    let mut o = banner(color);
    o.push('\n');
    o.push_str("  rune                        start the REPL\n");
    o.push_str("  rune eval <expr>            evaluate one expression\n");
    o.push_str("  rune run <file>             interpret a program\n");
    o.push_str("  rune fmt [--to=…] <file>    format between registers (rust | rune)\n");
    o.push_str("  rune check <file>           type-check (mold) without running\n");
    o.push('\n');
    o.push_str(&g("  rune help                   print this help\n"));
    o.push_str(&g("  rune --version              print version\n"));
    o
}

/// The meta-command list shown by `:help` inside the REPL.
pub fn repl_help(color: bool) -> String {
    let g = |s: &str| paint(color, GR, s);
    let mut o = String::new();
    o.push_str(&g("  :help          this list\n"));
    o.push_str(&g("  :env           show accumulated bindings\n"));
    o.push_str(&g("  :reset         clear all bindings\n"));
    o.push_str(&g("  :quit          leave the REPL  (or Ctrl-D)\n"));
    o.push('\n');
    o.push_str(&g("  let x = 5      bind a name; it persists for later lines\n"));
    o.push_str(&g("  x * 2         evaluate an expression against the bindings\n"));
    o
}
