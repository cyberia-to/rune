/// Subject model for rune.
/// The subject is a Nox noun containing the entire visible environment.
/// Every name in a rune program resolves through the subject via tree-slot lookup.
///
/// Subject layout (right-nested cons list):
///   [~self [~now [~here [~caps [~code [~libs [~mem ~world]]]]]]]
///
/// Axis assignments (cyber Nox slot navigation):
///   2   = ~self   — neuron identity (@neuron)
///   6   = ~now    — current time (@da)
///   14  = ~here   — current graph location (#)
///   30  = ~caps   — capability set
///   62  = ~code   — the running program core
///   126 = ~libs   — imported library cores
///   254 = ~mem    — persistent state (arbitrary noun)
///   255 = ~world  — cybergraph slice (visible particles and links)
use rune_ast::Noun;

/// A fully-populated subject noun.
pub struct Subject {
    pub self_neuron: Noun,
    pub now: Noun,
    pub here: Noun,
    pub caps: Noun,
    pub code: Noun,
    pub libs: Noun,
    pub mem: Noun,
    pub world: Noun,
}

impl Subject {
    /// Construct the subject noun: a right-nested cell tree.
    /// Layout: [~self [~now [~here [~caps [~code [~libs [~mem ~world]]]]]]]
    pub fn to_noun(&self) -> Noun {
        // Build from inside out: [mem world], then [libs [mem world]], etc.
        let inner7 = Noun::cell(self.mem.clone(), self.world.clone());
        let inner6 = Noun::cell(self.libs.clone(), inner7);
        let inner5 = Noun::cell(self.code.clone(), inner6);
        let inner4 = Noun::cell(self.caps.clone(), inner5);
        let inner3 = Noun::cell(self.here.clone(), inner4);
        let inner2 = Noun::cell(self.now.clone(), inner3);
        Noun::cell(self.self_neuron.clone(), inner2)
    }

    /// Minimal subject for standalone evaluation (no graph access).
    pub fn minimal() -> Self {
        Subject {
            self_neuron: Noun::Atom(0),
            now: Noun::Atom(0),
            here: Noun::Atom(0),
            caps: Noun::Atom(0),
            code: Noun::Atom(0),
            libs: Noun::Atom(0),
            mem: Noun::Atom(0),
            world: Noun::Atom(0),
        }
    }
}

/// Named slot axes for use in lowering and debugging.
///
/// Subject structure: [s0 [s1 [s2 [s3 [s4 [s5 [s6 s7]]]]]]]
/// Slots 0–6: head at each nesting level → axis = (1 << (n+2)) - 2
/// Slot 7: tail at the 7th level → axis 255
pub mod axis {
    pub const SELF:  u64 = 2;
    pub const NOW:   u64 = 6;
    pub const HERE:  u64 = 14;
    pub const CAPS:  u64 = 30;
    pub const CODE:  u64 = 62;
    pub const LIBS:  u64 = 126;
    pub const MEM:   u64 = 254;
    pub const WORLD: u64 = 255;

    /// Axis for the n-th slot in a right-nested cons list (0-indexed).
    /// Slots 0..6: head at each nesting level → (1 << (n+2)) - 2
    /// Slot 7: tail at the 7th level → 255
    pub fn subject_slot_axis(n: usize) -> u64 {
        if n < 7 { (1u64 << (n + 2)) - 2 } else { 255 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune_interp::axis as nox_axis;

    fn a(n: u64) -> Noun { Noun::Atom(n) }

    /// Build a distinct subject where each slot holds a unique atom,
    /// then verify that each axis constant navigates to the correct value.
    #[test]
    fn axis_constants_are_correct() {
        let subj = Subject {
            self_neuron: a(10),
            now:         a(11),
            here:        a(12),
            caps:        a(13),
            code:        a(14),
            libs:        a(15),
            mem:         a(16),
            world:       a(17),
        };
        let noun = subj.to_noun();

        let get = |ax: u64| nox_axis(&noun, ax).expect("axis failed");

        assert_eq!(get(axis::SELF),  a(10), "SELF axis {}", axis::SELF);
        assert_eq!(get(axis::NOW),   a(11), "NOW axis {}",  axis::NOW);
        assert_eq!(get(axis::HERE),  a(12), "HERE axis {}", axis::HERE);
        assert_eq!(get(axis::CAPS),  a(13), "CAPS axis {}", axis::CAPS);
        assert_eq!(get(axis::CODE),  a(14), "CODE axis {}", axis::CODE);
        assert_eq!(get(axis::LIBS),  a(15), "LIBS axis {}", axis::LIBS);
        assert_eq!(get(axis::MEM),   a(16), "MEM axis {}",  axis::MEM);
        assert_eq!(get(axis::WORLD), a(17), "WORLD axis {}", axis::WORLD);
    }

    /// subject_slot_axis(n) must agree with the named constants.
    #[test]
    fn subject_slot_axis_matches_constants() {
        assert_eq!(axis::subject_slot_axis(0), axis::SELF);
        assert_eq!(axis::subject_slot_axis(1), axis::NOW);
        assert_eq!(axis::subject_slot_axis(2), axis::HERE);
        assert_eq!(axis::subject_slot_axis(3), axis::CAPS);
        assert_eq!(axis::subject_slot_axis(4), axis::CODE);
        assert_eq!(axis::subject_slot_axis(5), axis::LIBS);
        assert_eq!(axis::subject_slot_axis(6), axis::MEM);
        assert_eq!(axis::subject_slot_axis(7), axis::WORLD);
    }
}
