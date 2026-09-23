//! Bounded reduction with deep effect suspension and portable checkpoints.
mod checkpoint;
use crate::{atom_u64, codec, err, pair, InterpError};
use rune_ast::Noun;

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub steps: u64,
    pub nodes: usize,
    pub depth: usize,
    pub frames: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            steps: 1_000_000,
            nodes: 65_536,
            depth: 128,
            frames: 256,
        }
    }
}

#[derive(Debug, Clone)]
enum Frame {
    Eval(Noun, Noun),
    Unary(u64),
    Binary(u64),
    Branch(Noun, Noun, Noun),
    Request(u64, Noun, Noun),
}
#[derive(Debug, Clone)]
struct Waiting {
    tag: u64,
    arguments: Noun,
    subject: Noun,
    body: Noun,
}

#[derive(Debug)]
pub enum Step {
    Done(Noun),
    Act { tag: u64, arguments: Noun },
    Event { tag: u64, selector: Noun },
    Yield,
}

#[derive(Debug)]
pub struct Machine {
    work: Vec<Frame>,
    values: Vec<Noun>,
    waiting: Option<Waiting>,
    used: u64,
    limits: Limits,
}

impl Machine {
    pub fn new(subject: Noun, formula: Noun, limits: Limits) -> Result<Self, InterpError> {
        let machine = Self {
            work: vec![Frame::Eval(subject, formula)],
            values: Vec::new(),
            waiting: None,
            used: 0,
            limits,
        };
        machine.check()?;
        Ok(machine)
    }
    pub fn used_steps(&self) -> u64 {
        self.used
    }
    pub fn run(&mut self, slice: u64) -> Result<Step, InterpError> {
        self.check()?;
        if slice == 0 {
            return Err(err("empty runtime slice"));
        }
        for _ in 0..slice {
            if let Some(step) = self.boundary()? {
                return Ok(step);
            }
            if self.used >= self.limits.steps {
                return Err(err("runtime step budget exhausted"));
            }
            self.used += 1;
            let frame = self.work.pop().ok_or_else(|| err("empty machine"))?;
            self.execute(frame)?;
            self.check()?;
        }
        Ok(self.boundary()?.unwrap_or(Step::Yield))
    }
    fn boundary(&self) -> Result<Option<Step>, InterpError> {
        if let Some(wait) = &self.waiting {
            return Ok(Some(if rune_ast::act::is_act(wait.tag) {
                Step::Act {
                    tag: wait.tag,
                    arguments: wait.arguments.clone(),
                }
            } else {
                Step::Event {
                    tag: wait.tag,
                    selector: wait.arguments.clone(),
                }
            }));
        }
        if self.work.is_empty() {
            if self.values.len() != 1 {
                return Err(err("invalid completed machine"));
            }
            return Ok(Some(Step::Done(self.values[0].clone())));
        }
        Ok(None)
    }
    pub fn resume(&mut self, value: Noun) -> Result<(), InterpError> {
        codec::measure(&value, self.limits.nodes, self.limits.depth)?;
        let wait = self
            .waiting
            .take()
            .ok_or_else(|| err("machine is not awaiting input"))?;
        self.work
            .push(Frame::Eval(Noun::cell(value, wait.subject), wait.body));
        self.check()
    }
    fn pop(&mut self) -> Result<Noun, InterpError> {
        self.values
            .pop()
            .ok_or_else(|| err("machine value underflow"))
    }
    fn execute(&mut self, frame: Frame) -> Result<(), InterpError> {
        match frame {
            Frame::Eval(subject, formula) => self.evaluate(subject, formula)?,
            Frame::Unary(op) => {
                let value = self.pop()?;
                let result = match op {
                    8 => crate::inv_field(atom_u64(&value, "inverse")?),
                    13 => !atom_u64(&value, "not")?,
                    15 => crate::hash_noun(&value),
                    _ => return Err(err("invalid unary frame")),
                };
                self.values.push(Noun::Atom(result));
            }
            Frame::Binary(op) => {
                let right = self.pop()?;
                let left = self.pop()?;
                match op {
                    2 => self.work.push(Frame::Eval(left, right)),
                    3 => self.values.push(Noun::cell(left, right)),
                    9 => self.values.push(Noun::Atom(u64::from(left != right))),
                    17 => self.values.push(crate::scry_world(&left, &right)),
                    _ => {
                        let a = atom_u64(&left, "binary left")?;
                        let b = atom_u64(&right, "binary right")?;
                        let value = match op {
                            5 => crate::add_field(a, b),
                            6 => crate::sub_field(
                                a % crate::GOLDILOCKS_PRIME,
                                b % crate::GOLDILOCKS_PRIME,
                            ),
                            7 => crate::mul_field(a, b),
                            10 => u64::from(a >= b),
                            11 => a ^ b,
                            12 => a & b,
                            14 => a << (b & 63),
                            _ => return Err(err("invalid binary frame")),
                        };
                        self.values.push(Noun::Atom(value));
                    }
                }
            }
            Frame::Branch(subject, yes, no) => {
                let value = self.pop()?;
                self.work.push(Frame::Eval(
                    subject,
                    if atom_u64(&value, "branch")? == 0 {
                        yes
                    } else {
                        no
                    },
                ));
            }
            Frame::Request(tag, subject, body) => {
                let arguments = self.pop()?;
                self.waiting = Some(Waiting {
                    tag,
                    arguments,
                    subject,
                    body,
                });
            }
        }
        Ok(())
    }
    fn evaluate(&mut self, subject: Noun, formula: Noun) -> Result<(), InterpError> {
        let (head, rest) = pair(&formula)?;
        let op = if matches!(head, Noun::Cell(..)) {
            self.binary(3, subject, head, rest);
            return Ok(());
        } else {
            atom_u64(&head, "opcode")?
        };
        match op {
            0 => self
                .values
                .push(crate::axis(&subject, atom_u64(&rest, "axis")?)?),
            1 => self.values.push(rest),
            2 | 3 | 5 | 6 | 7 | 9 | 10 | 11 | 12 | 14 | 17 => {
                let (a, b) = pair(&rest)?;
                self.binary(op, subject, a, b);
            }
            4 => {
                let (test, alternatives) = pair(&rest)?;
                let (yes, no) = pair(&alternatives)?;
                self.work.push(Frame::Branch(subject.clone(), yes, no));
                self.work.push(Frame::Eval(subject, test));
            }
            8 | 13 | 15 => {
                self.work.push(Frame::Unary(op));
                self.work.push(Frame::Eval(subject, rest));
            }
            16 => {
                let (meta, body) = pair(&rest)?;
                let (tag, arguments) = pair(&meta)?;
                let tag = atom_u64(&tag, "hint/act tag")?;
                self.work.push(Frame::Request(tag, subject.clone(), body));
                self.work.push(Frame::Eval(subject, arguments));
            }
            _ => return Err(err("unsupported runtime opcode")),
        }
        Ok(())
    }
    fn binary(&mut self, op: u64, subject: Noun, left: Noun, right: Noun) {
        self.work.push(Frame::Binary(op));
        self.work.push(Frame::Eval(subject.clone(), right));
        self.work.push(Frame::Eval(subject, left));
    }
    fn check(&self) -> Result<(), InterpError> {
        if self.limits.nodes == 0
            || self.limits.nodes > 1_048_576
            || self.limits.depth == 0
            || self.limits.depth > 256
            || self.limits.frames == 0
            || self.limits.frames > 256
            || self.used > self.limits.steps
            || self.work.len() + self.values.len() > self.limits.frames
        {
            return Err(err("machine resource limit"));
        }
        let mut used = 0usize;
        let mut count = |value: &Noun| -> Result<(), InterpError> {
            used = used
                .checked_add(codec::measure(value, self.limits.nodes, self.limits.depth)?)
                .ok_or_else(|| err("machine size overflow"))?;
            if used > self.limits.nodes {
                return Err(err("machine data budget exhausted"));
            }
            Ok(())
        };
        for frame in &self.work {
            match frame {
                Frame::Eval(a, b) | Frame::Request(_, a, b) => {
                    count(a)?;
                    count(b)?;
                }
                Frame::Branch(a, b, c) => {
                    count(a)?;
                    count(b)?;
                    count(c)?;
                }
                _ => (),
            }
        }
        for value in &self.values {
            count(value)?;
        }
        if let Some(wait) = &self.waiting {
            count(&wait.arguments)?;
            count(&wait.subject)?;
            count(&wait.body)?;
        }
        Ok(())
    }
}
