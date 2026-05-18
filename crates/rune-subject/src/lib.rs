/// Subject model for rune.
/// The subject is a Nox noun containing the entire visible environment.
/// Every name in a rune program resolves through the subject via tree-slot lookup.
///
/// Subject layout (axis assignments):
///   2  = ~self   — neuron identity (@neuron)
///   6  = ~now    — current time (@da)
///   10 = ~here   — current graph location (#)
///   14 = ~caps   — capability set
///   18 = ~code   — the running program core
///   22 = ~libs   — imported library cores
///   26 = ~mem    — persistent state (arbitrary noun)
///   30 = ~world  — cybergraph slice (visible particles and links)
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
        let inner = Noun::cell(self.mem.clone(), self.world.clone());
        let inner = Noun::cell(self.libs.clone(), inner);
        let inner = Noun::cell(self.code.clone(), inner);
        let inner = Noun::cell(self.caps.clone(), inner);
        let inner = Noun::cell(self.here.clone(), inner);
        let inner = Noun::cell(self.now.clone(), inner);
        Noun::cell(self.self_neuron.clone(), inner)
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
pub mod axis {
    pub const SELF:  u64 = 2;
    pub const NOW:   u64 = 6;
    pub const HERE:  u64 = 10;
    pub const CAPS:  u64 = 14;
    pub const CODE:  u64 = 18;
    pub const LIBS:  u64 = 22;
    pub const MEM:   u64 = 26;
    pub const WORLD: u64 = 30;
}
