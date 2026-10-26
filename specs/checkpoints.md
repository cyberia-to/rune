---
title: bounded interpreter checkpoints
tags: rune, neuron, prog, spec
status: implementation
---
# bounded interpreter checkpoints

The ephemeral reactive gate driver is `event::run_events`. Its state/subject
layout and trusted base authority are unchanged. `run_cell` is a deprecated 0.1
forwarder scheduled for removal in 0.2; it defines no runtime identity. Durable
prog/job recovery is owned by the neuron adapter through the bounded machine.

The machine adapter is an explicit reduction stack with finite step, data-node,
depth and frame limits. It evaluates the interpreter's opcodes 0–17. Its ABI pins
the existing interpreter arithmetic, word bit operations and legacy hash result;
this adapter claims host-observed execution, with no Nox proof equivalence claim.

Every nested opcode-16 request evaluates its arguments, then parks the complete
evaluation stack. Recognized act tags yield Act; other atom tags yield Event.
The caller persists the request/checkpoint before an external effect. Resume
injects the recorded result at the saved subject head. It consumes one wait.
Yield returns a checkpoint after a bounded slice. Total charged steps persist.

Checkpoint revision 1 stores version, used steps, reduction frames, value stack
and pending request as a bounded noun list. Its binary codec uses 0 + eight
little-endian bytes for a word and 1 + two recursively framed children for a
pair, in preorder. This is a runtime artifact codec, independently named and
hashed by its content codec. Decode rejects unknown tags, missing/trailing data,
excess depth/nodes, unknown frames and incompatible revisions.

The host supplies limits again on restore and verifies the artifact against the
owning definition/history. Checkpoint bytes contain no live authority handle.
The machine never dispatches effects itself. The synchronous eval_with_authority
API binds host authority outside subject rewriting; run_events binds from its
mounted base before constructing event/state subjects.

Logical node/frame limits bound interpreter state; a reduction can temporarily
copy a bounded operand. Physical process memory limits belong to body/placement.
Callers requiring a hard physical ceiling use an enforced process boundary.

Classic parsing consumes the whole program, applies arithmetic precedence, and
allows subject addresses inside expressions. parse_bounded additionally limits
tokens; the parser limits expression nesting to 64. lower_bounded accounts for
gate/door expansion; subject slots include lexical stack prefixes and lowering
rejects excessive lexical/recursive depth before constructing invalid axes.
host(request) is the classic unary act builtin for a packed executor request.
The legacy Expr::Host target field is not a routing contract for this builtin.
