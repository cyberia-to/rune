use super::{Frame, Limits, Machine, Waiting};
use crate::{atom_u64, codec, err, InterpError};
use rune_ast::Noun;

fn a(n: u64) -> Noun {
    Noun::Atom(n)
}
fn list(values: impl IntoIterator<Item = Noun>) -> Noun {
    let values: Vec<_> = values.into_iter().collect();
    values
        .into_iter()
        .rev()
        .fold(a(0), |tail, head| Noun::cell(head, tail))
}
fn items(value: Noun, limit: usize) -> Result<Vec<Noun>, InterpError> {
    let mut values = Vec::new();
    let mut rest = value;
    loop {
        match rest {
            Noun::Atom(0) => return Ok(values),
            Noun::Cell(h, t) if values.len() < limit => {
                values.push(*h);
                rest = *t;
            }
            _ => return Err(err("invalid checkpoint list")),
        }
    }
}
fn frame(value: &Frame) -> Noun {
    match value {
        Frame::Eval(a1, b) => list([a(0), a1.clone(), b.clone()]),
        Frame::Unary(op) => list([a(1), a(*op)]),
        Frame::Binary(op) => list([a(2), a(*op)]),
        Frame::Branch(s, y, n) => list([a(3), s.clone(), y.clone(), n.clone()]),
        Frame::Request(tag, s, b) => list([a(4), a(*tag), s.clone(), b.clone()]),
    }
}
fn read_frame(value: Noun) -> Result<Frame, InterpError> {
    let fields = items(value, 4)?;
    Ok(match fields.as_slice() {
        [Noun::Atom(0), s, f] => Frame::Eval(s.clone(), f.clone()),
        [Noun::Atom(1), Noun::Atom(op @ (8 | 13 | 15))] => Frame::Unary(*op),
        [Noun::Atom(2), Noun::Atom(op @ (2 | 3 | 5 | 6 | 7 | 9 | 10 | 11 | 12 | 14 | 17))] => {
            Frame::Binary(*op)
        }
        [Noun::Atom(3), s, y, n] => Frame::Branch(s.clone(), y.clone(), n.clone()),
        [Noun::Atom(4), Noun::Atom(tag), s, b] => Frame::Request(*tag, s.clone(), b.clone()),
        _ => return Err(err("invalid checkpoint frame")),
    })
}
impl Machine {
    pub fn checkpoint(&self) -> Result<Vec<u8>, InterpError> {
        self.check()?;
        let wait = match &self.waiting {
            Some(w) => list([
                a(w.tag),
                w.arguments.clone(),
                w.subject.clone(),
                w.body.clone(),
            ]),
            None => a(0),
        };
        let value = list([
            a(1),
            a(self.used),
            list(self.work.iter().map(frame)),
            list(self.values.clone()),
            wait,
        ]);
        codec::encode(
            &value,
            self.limits.nodes + self.limits.frames * 16 + 64,
            self.limits.depth + self.limits.frames + 16,
        )
    }
    pub fn restore(bytes: &[u8], limits: Limits) -> Result<Self, InterpError> {
        // Validate limits before using them in allocation/decoder bounds.
        Self::new(a(0), Noun::cell(a(1), a(0)), limits)?;
        let value = codec::decode(
            bytes,
            limits.nodes + limits.frames * 16 + 64,
            limits.depth + limits.frames + 16,
        )?;
        let mut fields = items(value, 5)?.into_iter();
        if fields.next() != Some(a(1)) {
            return Err(err("unsupported checkpoint revision"));
        }
        let used = atom_u64(
            &fields
                .next()
                .ok_or_else(|| err("checkpoint steps missing"))?,
            "steps",
        )?;
        let work = items(
            fields
                .next()
                .ok_or_else(|| err("checkpoint work missing"))?,
            limits.frames,
        )?
        .into_iter()
        .map(read_frame)
        .collect::<Result<Vec<_>, _>>()?;
        let values = items(
            fields
                .next()
                .ok_or_else(|| err("checkpoint values missing"))?,
            limits.frames,
        )?;
        let waiting = match items(
            fields
                .next()
                .ok_or_else(|| err("checkpoint wait missing"))?,
            4,
        )?
        .as_slice()
        {
            [] => None,
            [Noun::Atom(tag), arguments, subject, body] => Some(Waiting {
                tag: *tag,
                arguments: arguments.clone(),
                subject: subject.clone(),
                body: body.clone(),
            }),
            _ => return Err(err("invalid checkpoint wait")),
        };
        let machine = Self {
            work,
            values,
            waiting,
            used,
            limits,
        };
        machine.check()?;
        Ok(machine)
    }
}
