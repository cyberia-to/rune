//! rune ↔ prysm bridge.
//!
//! A rune program that renders UI evaluates to a **chunk-noun** — a noun tree
//! using the `rune_ast::tag` ABI. This crate decodes that noun into
//! [`tape::Chunk`]s, the exact wire format prysm's `dispatch` renders (to wgpu
//! cells, ansi, or html). It is the only coupling between rune and prysm: rune
//! emits nouns, prysm renders chunks, and `noun_to_chunks` is the seam.
//!
//! ```text
//!   rune program ──lower──> Nox formula ──eval──> chunk-noun
//!                                                     │  noun_to_chunks
//!                                                     ▼
//!                                            Vec<tape::Chunk> ──StreamChannel──> prysm::dispatch
//! ```

use rune_ast::{tag, Noun};
use tape::{bytes::Bytes, encode_nested, render, sigil, Chunk};

/// Decode a `tape`/cord payload noun into raw bytes.
///
/// Two encodings are accepted:
/// - **tape**: a null-terminated cons list of byte atoms `[h [e … 0]]` (how
///   `rune-lower` emits text — handles arbitrary length over `u64` atoms).
/// - **cord**: a single atom, read as little-endian bytes with trailing NULs
///   trimmed (a convenience for short literals).
pub fn noun_to_bytes(n: &Noun) -> Vec<u8> {
    match n {
        Noun::Atom(0) => Vec::new(),
        Noun::Atom(a) => {
            let mut v = a.to_le_bytes().to_vec();
            while v.last() == Some(&0) {
                v.pop();
            }
            v
        }
        Noun::Cell(_, _) => {
            let mut out = Vec::new();
            let mut cur = n;
            loop {
                match cur {
                    Noun::Cell(h, t) => {
                        if let Noun::Atom(b) = h.as_ref() {
                            out.push(*b as u8);
                        }
                        cur = t.as_ref();
                    }
                    Noun::Atom(0) => break,
                    Noun::Atom(b) => {
                        out.push(*b as u8);
                        break;
                    }
                }
            }
            out
        }
    }
}

fn text_of(n: &Noun) -> String {
    String::from_utf8_lossy(&noun_to_bytes(n)).into_owned()
}

fn tr_cells(payload: &Noun) -> Vec<Chunk> {
    let mut cells = Vec::new();
    let mut cur = payload;
    while let Noun::Cell(elem, rest) = cur {
        cells.push(Chunk::text(&text_of(elem)));
        cur = rest.as_ref();
    }
    cells
}

fn table_rows(payload: &Noun) -> Vec<Chunk> {
    let mut out = Vec::new();
    let mut first = true;
    let mut cur = payload;
    loop {
        match cur {
            Noun::Cell(elem, rest) => {
                if let Noun::Cell(h, tail) = elem.as_ref() {
                    if matches!(h.as_ref(), Noun::Atom(t) if *t == tag::LIST) {
                        out.extend(table_rows(tail));
                        cur = rest.as_ref();
                        continue;
                    }
                    if matches!(h.as_ref(), Noun::Atom(t) if *t == tag::TR) {
                        let cells = tr_cells(tail);
                        let (sig, rend) = if first {
                            first = false;
                            (sigil::FAS, render::STRUCT)
                        } else {
                            (sigil::COL, render::STRUCT)
                        };
                        out.push(Chunk::new(sig, rend, encode_nested(&cells)));
                    }
                }
                cur = rest.as_ref();
            }
            _ => break,
        }
    }
    out
}

/// Decode a single prysm element noun `[tag payload]` into one [`Chunk`].
/// Returns `None` if the noun is not a recognised element.
pub fn noun_to_chunk(n: &Noun) -> Option<Chunk> {
    let Noun::Cell(head, payload) = n else {
        return None;
    };
    let Noun::Atom(t) = head.as_ref() else {
        return None;
    };
    let chunk = match *t {
        tag::TEXT => Chunk::text(&text_of(payload)),
        tag::ANNO => Chunk::annotation(&text_of(payload)),
        tag::ERROR => Chunk::error(&text_of(payload)),
        tag::LOG => Chunk::new(sigil::DOT, render::LOG, Bytes::from(noun_to_bytes(payload))),
        tag::BUTTON => {
            // payload = [label-tape target-tape]
            let Noun::Cell(label, target) = payload.as_ref() else {
                return None;
            };
            let label_c = Chunk::annotation(&text_of(label)); // (~, t)
            let target_c = Chunk::text(&text_of(target)); // (#, t)
            Chunk::new(
                sigil::ZAP,
                render::COMPONENT,
                encode_nested(&[label_c, target_c]),
            )
        }
        tag::STAT => {
            let Noun::Cell(value, caption) = payload.as_ref() else {
                return None;
            };
            let value_c = Chunk::text(&text_of(value));
            let cap_c = Chunk::annotation(&text_of(caption));
            Chunk::new(
                sigil::BUC,
                render::COMPONENT,
                encode_nested(&[value_c, cap_c]),
            )
        }
        tag::ROW => {
            let mut kids = Vec::new();
            let mut cur = payload.as_ref();
            while let Noun::Cell(elem, rest) = cur {
                if let Some(c) = noun_to_chunk(elem) {
                    kids.push(c);
                }
                cur = rest.as_ref();
            }
            Chunk::new(sigil::LUS, render::COMPONENT, encode_nested(&kids))
        }
        tag::TABLE => Chunk::new(
            sigil::HAX,
            render::TABLE,
            encode_nested(&table_rows(payload)),
        ),
        tag::TR => {
            let mut cells = Vec::new();
            let mut cur = payload.as_ref();
            while let Noun::Cell(elem, rest) = cur {
                cells.push(Chunk::text(&text_of(elem)));
                cur = rest.as_ref();
            }
            Chunk::new(sigil::COL, render::STRUCT, encode_nested(&cells))
        }
        _ => return None,
    };
    Some(chunk)
}

/// Decode a rune result noun into a stream of [`Chunk`]s.
///
/// A `[LIST e1 [e2 … 0]]` noun yields one chunk per element; any other
/// recognised element noun yields a single chunk; anything else yields none.
pub fn noun_to_chunks(n: &Noun) -> Vec<Chunk> {
    if let Noun::Cell(head, tail) = n {
        if matches!(head.as_ref(), Noun::Atom(t) if *t == tag::LIST) {
            let mut out = Vec::new();
            let mut cur = tail.as_ref();
            while let Noun::Cell(elem, rest) = cur {
                if let Noun::Cell(h, _) = elem.as_ref() {
                    if matches!(h.as_ref(), Noun::Atom(t) if *t == tag::LIST) {
                        out.extend(noun_to_chunks(elem));
                        cur = rest.as_ref();
                        continue;
                    }
                }
                if let Some(c) = noun_to_chunk(elem) {
                    out.push(c);
                }
                cur = rest.as_ref();
            }
            return out;
        }
    }
    noun_to_chunk(n).into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a tape noun the way rune-lower does, for fixtures.
    fn tape(s: &str) -> Noun {
        let mut n = Noun::Atom(0);
        for &b in s.as_bytes().iter().rev() {
            n = Noun::cell(Noun::Atom(b as u64), n);
        }
        n
    }

    #[test]
    fn tape_roundtrips_arbitrary_length() {
        // 8+ bytes — the case a single u64 atom cannot hold.
        let s = "hello, cyber neuron";
        assert_eq!(noun_to_bytes(&tape(s)), s.as_bytes());
    }

    #[test]
    fn cord_atom_decodes_le() {
        // "hi" packed LSB-first into one atom = 0x6968.
        assert_eq!(noun_to_bytes(&Noun::Atom(0x6968)), b"hi");
        assert_eq!(noun_to_bytes(&Noun::Atom(0)), b"");
    }

    #[test]
    fn text_element_is_hax_text() {
        let n = Noun::cell(Noun::Atom(tag::TEXT), tape("hi"));
        let c = noun_to_chunk(&n).unwrap();
        assert_eq!((c.sigil, c.render), (sigil::HAX, render::TEXT));
        assert_eq!(&c.payload[..], b"hi");
    }

    #[test]
    fn error_element_is_zap_error() {
        let n = Noun::cell(Noun::Atom(tag::ERROR), tape("boom"));
        let c = noun_to_chunk(&n).unwrap();
        assert_eq!((c.sigil, c.render), (sigil::ZAP, render::ERROR));
        assert!(std::str::from_utf8(&c.payload).unwrap().contains("boom"));
    }

    #[test]
    fn button_element_is_zap_component_with_label_and_target() {
        // [BUTTON [label target]]
        let n = Noun::cell(
            Noun::Atom(tag::BUTTON),
            Noun::cell(tape("ok"), tape("@master")),
        );
        let c = noun_to_chunk(&n).unwrap();
        assert_eq!((c.sigil, c.render), (sigil::ZAP, render::COMPONENT));
        // Inner: annotation(label) + text(target) — matches prysm action::spawn.
        let inner = tape::decode_nested(&c.payload);
        assert_eq!(inner.len(), 2);
        assert_eq!(
            (inner[0].sigil, inner[0].render),
            (sigil::SIG, render::TEXT)
        );
        assert_eq!(&inner[0].payload[..], b"ok");
        assert_eq!(
            (inner[1].sigil, inner[1].render),
            (sigil::HAX, render::TEXT)
        );
        assert_eq!(&inner[1].payload[..], b"@master");
    }

    #[test]
    fn list_yields_one_chunk_per_element() {
        // [LIST text("a") [error("b") 0]]
        let elems = Noun::cell(
            Noun::cell(Noun::Atom(tag::TEXT), tape("a")),
            Noun::cell(Noun::cell(Noun::Atom(tag::ERROR), tape("b")), Noun::Atom(0)),
        );
        let n = Noun::cell(Noun::Atom(tag::LIST), elems);
        let chunks = noun_to_chunks(&n);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].render, render::TEXT);
        assert_eq!(chunks[1].render, render::ERROR);
    }

    #[test]
    fn stat_element_is_buc_component() {
        let n = Noun::cell(
            Noun::Atom(tag::STAT),
            Noun::cell(tape("12"), tape("particles")),
        );
        let c = noun_to_chunk(&n).unwrap();
        assert_eq!((c.sigil, c.render), (sigil::BUC, render::COMPONENT));
        let inner = tape::decode_nested(&c.payload);
        assert_eq!(inner.len(), 2);
        assert_eq!(&inner[0].payload[..], b"12");
        assert_eq!(&inner[1].payload[..], b"particles");
    }

    #[test]
    fn nested_list_splices_into_col() {
        let inner = Noun::cell(Noun::cell(Noun::Atom(tag::TEXT), tape("x")), Noun::Atom(0));
        let elems = Noun::cell(Noun::cell(Noun::Atom(tag::LIST), inner), Noun::Atom(0));
        let n = Noun::cell(Noun::Atom(tag::LIST), elems);
        let chunks = noun_to_chunks(&n);
        assert_eq!(chunks.len(), 1);
        assert_eq!(&chunks[0].payload[..], b"x");
    }

    #[test]
    fn table_first_tr_is_header() {
        let header = Noun::cell(Noun::Atom(tag::TR), Noun::cell(tape("name"), Noun::Atom(0)));
        let n = Noun::cell(Noun::Atom(tag::TABLE), Noun::cell(header, Noun::Atom(0)));
        let c = noun_to_chunk(&n).unwrap();
        assert_eq!((c.sigil, c.render), (sigil::HAX, render::TABLE));
        let rows = tape::decode_nested(&c.payload);
        assert_eq!(rows.len(), 1);
        assert_eq!(
            (rows[0].sigil, rows[0].render),
            (sigil::FAS, render::STRUCT)
        );
    }
}
