//! ANSI rendering of prysm chunks for the terminal.
//!
//! tade carries no terminal renderer — rendering lives in prysm, which targets
//! wgpu/Bevy — so the CLI owns this mapping: `(sigil, render)` → a styled line.
//! Colour is gated on `color` (set from tty detection) so piped output stays
//! plain text.

use tade::{decode_nested, read_kv, render, sigil, Chunk};

pub const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";
const GRAY: &str = "\x1b[90m";
const RED: &str = "\x1b[31m";
const CYAN: &str = "\x1b[36m";

/// Wrap `s` in `style … RESET` when `color`, else return it unchanged.
fn paint(color: bool, style: &str, s: &str) -> String {
    if color {
        format!("{style}{s}{RESET}")
    } else {
        s.to_string()
    }
}

fn text_of(c: &Chunk) -> String {
    String::from_utf8_lossy(&c.payload).into_owned()
}

/// Render one chunk to a styled line.
pub fn ansi_chunk(c: &Chunk, color: bool) -> String {
    match c.render {
        // text — plain, unless it is an annotation (sigil `~`), which is dim
        render::TEXT if c.sigil == sigil::SIG => paint(color, DIM, &text_of(c)),
        render::TEXT => text_of(c),

        // typed error — decode the molecule and pull out the message + source
        render::ERROR => {
            let kv = read_kv(&c.payload);
            let msg = kv.get("message").map(text_of).unwrap_or_default();
            let src = kv.get("source").map(text_of).unwrap_or_default();
            let body = if src.is_empty() {
                format!("✗ {msg}")
            } else {
                format!("✗ {msg}  ({src})")
            };
            paint(color, RED, &body)
        }

        // log line — `• [level] message`
        render::LOG => {
            let kv = read_kv(&c.payload);
            let lvl = kv.get("level").map(text_of).unwrap_or_default();
            let msg = kv.get("message").map(text_of).unwrap_or_default();
            paint(color, GRAY, &format!("• [{lvl}] {msg}"))
        }

        // button / component — `[ label ] → target`
        render::COMPONENT => {
            let inner = decode_nested(&c.payload);
            let label = inner.first().map(text_of).unwrap_or_default();
            let target = inner.get(1).map(text_of).unwrap_or_default();
            format!(
                "{}{}",
                paint(color, CYAN, &format!("[ {label} ]")),
                paint(color, GRAY, &format!(" → {target}")),
            )
        }

        // anything else — show the payload as text
        _ => text_of(c),
    }
}

/// Render a stream of chunks, one per line.
pub fn ansi_chunks(chunks: &[Chunk], color: bool) -> String {
    chunks
        .iter()
        .map(|c| ansi_chunk(c, color))
        .collect::<Vec<_>>()
        .join("\n")
}
