/// Mold system for rune.
/// A mold is a gate (Noun → Noun) that idempotently normalizes values to their type.
/// Molds are runtime values: composable, storable, passable.
/// The compiler also evaluates them statically for type checking.
use rune_ast::Noun;

/// A mold: a normalizing function from Noun to Noun.
/// `normalize` returns Ok(canonical) or Err if the value is outside the mold.
pub trait Mold: Send + Sync {
    fn name(&self) -> &str;
    fn normalize(&self, noun: &Noun) -> Result<Noun, MoldError>;
    fn check(&self, noun: &Noun) -> bool { self.normalize(noun).is_ok() }
}

#[derive(Debug)]
pub struct MoldError {
    pub message: String,
}

/// `@ud` — unsigned decimal: any atom is valid
pub struct UdMold;
impl Mold for UdMold {
    fn name(&self) -> &str { "@ud" }
    fn normalize(&self, noun: &Noun) -> Result<Noun, MoldError> {
        match noun {
            Noun::Atom(_) => Ok(noun.clone()),
            _ => Err(MoldError { message: "@ud: expected atom".into() }),
        }
    }
}

/// `@da` — absolute date: atom representing unix timestamp
pub struct DaMold;
impl Mold for DaMold {
    fn name(&self) -> &str { "@da" }
    fn normalize(&self, noun: &Noun) -> Result<Noun, MoldError> {
        match noun {
            Noun::Atom(_) => Ok(noun.clone()),
            _ => Err(MoldError { message: "@da: expected atom".into() }),
        }
    }
}

/// `#` — particle: any noun (content-addressed, always valid)
pub struct ParticleMold;
impl Mold for ParticleMold {
    fn name(&self) -> &str { "#" }
    fn normalize(&self, noun: &Noun) -> Result<Noun, MoldError> {
        Ok(noun.clone()) // any noun is a valid particle
    }
}

/// `@neuron` — neuron identity: atom (hemera hash of public key)
pub struct NeuronMold;
impl Mold for NeuronMold {
    fn name(&self) -> &str { "@neuron" }
    fn normalize(&self, noun: &Noun) -> Result<Noun, MoldError> {
        match noun {
            Noun::Atom(_) => Ok(noun.clone()),
            _ => Err(MoldError { message: "@neuron: expected atom".into() }),
        }
    }
}

/// Resolve a mold name to its implementation.
/// Algebra-specific molds (@nebu, @kuro, etc.) land when strata crate is integrated.
pub fn resolve(name: &str) -> Option<Box<dyn Mold>> {
    match name {
        "@ud"     | "ud"     => Some(Box::new(UdMold)),
        "@da"     | "da"     => Some(Box::new(DaMold)),
        "#"                  => Some(Box::new(ParticleMold)),
        "@neuron" | "neuron" => Some(Box::new(NeuronMold)),
        _ => None,
    }
}
