//! Bounded portable noun checkpoint codec. Framing is independent of data identity.
use crate::{err, InterpError};
use rune_ast::Noun;

pub fn measure(noun: &Noun, max_nodes: usize, max_depth: usize) -> Result<usize, InterpError> {
    let mut todo = vec![(noun, 0usize)];
    let mut count = 0usize;
    while let Some((value, depth)) = todo.pop() {
        count = count
            .checked_add(1)
            .ok_or_else(|| err("noun size overflow"))?;
        if count > max_nodes || depth > max_depth {
            return Err(err("noun resource limit"));
        }
        if let Noun::Cell(h, t) = value {
            todo.push((t, depth + 1));
            todo.push((h, depth + 1));
        }
    }
    Ok(count)
}

pub fn encode(noun: &Noun, max_nodes: usize, max_depth: usize) -> Result<Vec<u8>, InterpError> {
    measure(noun, max_nodes, max_depth)?;
    let mut bytes = Vec::new();
    let mut todo = vec![noun];
    while let Some(value) = todo.pop() {
        match value {
            Noun::Atom(n) => {
                bytes.push(0);
                bytes.extend_from_slice(&n.to_le_bytes());
            }
            Noun::Cell(h, t) => {
                bytes.push(1);
                todo.push(t);
                todo.push(h);
            }
        }
    }
    Ok(bytes)
}

pub fn decode(bytes: &[u8], max_nodes: usize, max_depth: usize) -> Result<Noun, InterpError> {
    let mut frames: Vec<Option<Noun>> = Vec::new();
    let mut pos = 0usize;
    let mut nodes = 0usize;
    loop {
        nodes = nodes
            .checked_add(1)
            .ok_or_else(|| err("noun size overflow"))?;
        if nodes > max_nodes || frames.len() > max_depth {
            return Err(err("noun resource limit"));
        }
        let tag = *bytes.get(pos).ok_or_else(|| err("truncated noun"))?;
        pos += 1;
        if tag == 1 {
            frames.push(None);
            continue;
        }
        if tag != 0 {
            return Err(err("unknown noun framing"));
        }
        let end = pos
            .checked_add(8)
            .ok_or_else(|| err("noun offset overflow"))?;
        let raw = bytes
            .get(pos..end)
            .ok_or_else(|| err("truncated noun atom"))?;
        let mut value = Noun::Atom(u64::from_le_bytes(
            raw.try_into().map_err(|_| err("invalid noun atom"))?,
        ));
        pos = end;
        loop {
            match frames.last_mut() {
                None => {
                    if pos != bytes.len() {
                        return Err(err("trailing noun bytes"));
                    }
                    return Ok(value);
                }
                Some(slot @ None) => {
                    *slot = Some(value);
                    break;
                }
                Some(Some(_)) => {
                    let head = frames
                        .pop()
                        .flatten()
                        .ok_or_else(|| err("invalid noun frame"))?;
                    value = Noun::cell(head, value);
                }
            }
        }
    }
}
