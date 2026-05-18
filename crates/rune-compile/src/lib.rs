//! rune compile back-end — background compilation and transparent hot-swap.
//!
//! Architecture:
//!   - CompileQueue: background thread pool for compilation jobs
//!   - ArtifactCache: source-hash → compiled artifact mapping
//!   - TierRouter: per-call-site tier (interpret | compile | compiled)
//!   - hot-swap: when compile finishes, next call uses compiled path
//!
//! The interpreter is always the fallback. Compilation is opportunistic.
//! Instant start is always preserved — compilation never blocks execution.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use rune_ast::Noun;

/// A compiled artifact — the result of optimizing a Nox formula.
/// In M6, this is the same Noun (no actual optimization yet).
/// In future milestones: TIR-optimized .nox bytecode.
#[derive(Clone)]
pub struct CompiledArtifact {
    /// The source formula hash (placeholder — hemera hash in M2+)
    pub source_hash: u64,
    /// The compiled formula (M6: same as source; future: optimized)
    pub formula: Noun,
    /// Whether this artifact was produced by the trident compile path
    pub via_trident: bool,
}

/// Per-callsite execution tier.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tier {
    /// Default: interpret directly
    Interpret,
    /// Submitted for compilation, still interpreting
    Compiling,
    /// Use compiled artifact
    Compiled,
}

/// Call counter per formula hash — tracks hot paths.
pub struct CallCounter {
    counts: HashMap<u64, u64>,
    threshold: u64,
}

impl CallCounter {
    pub fn new(threshold: u64) -> Self {
        CallCounter { counts: HashMap::new(), threshold }
    }

    /// Record a call. Returns true if this call crosses the compilation threshold.
    pub fn record(&mut self, formula_hash: u64) -> bool {
        let count = self.counts.entry(formula_hash).or_insert(0);
        *count += 1;
        *count == self.threshold
    }

    pub fn count(&self, formula_hash: u64) -> u64 {
        self.counts.get(&formula_hash).copied().unwrap_or(0)
    }
}

/// The artifact cache — maps source hash to compiled artifact.
pub struct ArtifactCache {
    inner: Arc<Mutex<HashMap<u64, CompiledArtifact>>>,
}

impl ArtifactCache {
    pub fn new() -> Self {
        ArtifactCache { inner: Arc::new(Mutex::new(HashMap::new())) }
    }

    pub fn get(&self, source_hash: u64) -> Option<CompiledArtifact> {
        self.inner.lock().unwrap().get(&source_hash).cloned()
    }

    pub fn store(&self, artifact: CompiledArtifact) {
        self.inner.lock().unwrap().insert(artifact.source_hash, artifact.clone());
    }
}

impl Default for ArtifactCache {
    fn default() -> Self { Self::new() }
}

/// Compile a formula. In M6 this is the trident path stub.
/// Returns an artifact immediately (background thread in M8+).
pub fn compile_formula(formula: &Noun, source_hash: u64) -> CompiledArtifact {
    // M6 stub: try to emit trident source and compile it
    // For now: return the formula unchanged with via_trident=false
    // Real trident integration: emit .tri source → trident::compile() → .nox
    CompiledArtifact {
        source_hash,
        formula: formula.clone(),
        via_trident: false,
    }
}

/// Attempt to compile a pure rune formula through trident.
/// Returns Some(artifact) if compilation succeeded, None if formula contains
/// dynamic forms (hint/host/eval) that trident cannot accept.
pub fn try_trident_compile(formula: &Noun, source_hash: u64) -> Option<CompiledArtifact> {
    // Check if formula is pure (no opcode 16/call or 17/look)
    if contains_dynamic(formula) {
        return None;
    }
    // M6 stub: real trident::compile() call would go here
    // When trident dep is added: parse formula as .tri, call trident::compile()
    Some(CompiledArtifact {
        source_hash,
        formula: formula.clone(),
        via_trident: false,  // will be true when real compile path lands
    })
}

/// Check if a formula contains dynamic opcodes (16=hint/call, 17=look).
/// Pure formulas are trident-compatible.
fn contains_dynamic(noun: &Noun) -> bool {
    match noun {
        Noun::Atom(_) => false,
        Noun::Cell(h, t) => {
            // opcode is atom head; if head is 16 or 17, it's dynamic
            if let Noun::Atom(op) = h.as_ref() {
                if *op == 16 || *op == 17 { return true; }
            }
            contains_dynamic(h) || contains_dynamic(t)
        }
    }
}

/// A simple formula hash — placeholder until hemera lands in M2.
pub fn formula_hash(noun: &Noun) -> u64 {
    match noun {
        Noun::Atom(n) => n.wrapping_mul(0x517cc1b727220a95),
        Noun::Cell(h, t) => {
            let hh = formula_hash(h);
            let th = formula_hash(t);
            hh.wrapping_mul(31).wrapping_add(th)
        }
    }
}

/// Cache of (formula_hash, subject_hash) → result.
/// Only used for pure formulas (no dynamic opcodes).
struct ResultCache {
    inner: HashMap<(u64, u64), Noun>,
    max_entries: usize,
}

impl ResultCache {
    fn new(max_entries: usize) -> Self {
        ResultCache { inner: HashMap::new(), max_entries }
    }

    fn get(&self, formula_hash: u64, subject_hash: u64) -> Option<&Noun> {
        self.inner.get(&(formula_hash, subject_hash))
    }

    fn store(&mut self, formula_hash: u64, subject_hash: u64, result: Noun) {
        if self.inner.len() < self.max_entries {
            self.inner.insert((formula_hash, subject_hash), result);
        }
        // When full: evict nothing (simple LRU-free policy for now)
        // Future: LRU eviction
    }

    fn len(&self) -> usize { self.inner.len() }
}

/// Stats snapshot returned by `TierRouter::stats`.
pub struct TierStats {
    pub result_cache_entries: usize,
}

/// TierRouter — makes the call-tier decision.
pub struct TierRouter {
    counter: CallCounter,
    cache: ArtifactCache,
    results: ResultCache,
    hot_threshold: u64,
}

impl TierRouter {
    pub fn new(compile_threshold: u64) -> Self {
        TierRouter {
            counter: CallCounter::new(compile_threshold),
            cache: ArtifactCache::new(),
            results: ResultCache::new(1024),
            hot_threshold: compile_threshold,
        }
    }

    /// Evaluate formula against subject, routing through the appropriate tier.
    pub fn eval(&mut self, subject: &Noun, formula: &Noun) -> Result<Noun, rune_interp::InterpError> {
        let fhash = formula_hash(formula);
        let is_pure = !contains_dynamic(formula);

        // 1. Check result cache (pure formulas only)
        if is_pure {
            let shash = formula_hash(subject);
            if let Some(cached) = self.results.get(fhash, shash) {
                return Ok(cached.clone());
            }
        }

        // 2. Check compiled artifact cache
        let formula_to_run = if let Some(artifact) = self.cache.get(fhash) {
            artifact.formula
        } else {
            formula.clone()
        };

        // 3. Interpret
        let result = rune_interp::eval(subject, &formula_to_run)?;

        // 4. Record call; on threshold: try to compile and cache artifact
        if self.counter.record(fhash) {
            if let Some(artifact) = try_trident_compile(formula, fhash) {
                self.cache.store(artifact);
            }
        }

        // 5. Memoize result for pure formulas that were called enough times
        if is_pure && self.counter.count(fhash) >= self.hot_threshold {
            let shash = formula_hash(subject);
            self.results.store(fhash, shash, result.clone());
        }

        Ok(result)
    }

    /// Returns stats useful for profiling and debugging.
    pub fn stats(&self) -> TierStats {
        TierStats {
            result_cache_entries: self.results.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune_ast::Noun;

    #[test]
    fn call_counter_threshold() {
        let mut c = CallCounter::new(3);
        assert!(!c.record(42));
        assert!(!c.record(42));
        assert!(c.record(42));  // third call hits threshold
        assert!(!c.record(42)); // fourth does not re-trigger
    }

    #[test]
    fn pure_formula_detection() {
        // [1 42] — quote literal, pure
        let pure = Noun::cell(Noun::Atom(1), Noun::Atom(42));
        assert!(!contains_dynamic(&pure));

        // [16 tag body] — hint, dynamic
        let dynamic = Noun::cell(Noun::Atom(16), Noun::cell(Noun::Atom(0), Noun::Atom(0)));
        assert!(contains_dynamic(&dynamic));
    }

    #[test]
    fn tier_router_interprets() {
        let mut router = TierRouter::new(100);
        let subj = Noun::Atom(0);
        let formula = Noun::cell(Noun::Atom(1), Noun::Atom(42));
        let result = router.eval(&subj, &formula).unwrap();
        assert_eq!(result, Noun::Atom(42));
    }

    #[test]
    fn result_cache_hit_after_threshold() {
        let mut router = TierRouter::new(3);
        let subj = Noun::Atom(0);
        // Pure formula: [1 42] = literal 42
        let formula = Noun::cell(Noun::Atom(1), Noun::Atom(42));

        // Call 3 times to hit threshold
        for _ in 0..3 {
            let r = router.eval(&subj, &formula).unwrap();
            assert_eq!(r, Noun::Atom(42));
        }

        // 4th call: result should come from cache
        let r = router.eval(&subj, &formula).unwrap();
        assert_eq!(r, Noun::Atom(42));
        assert_eq!(router.stats().result_cache_entries, 1);
    }

    #[test]
    fn dynamic_formula_not_memoized() {
        let mut router = TierRouter::new(2);
        let subj = Noun::Atom(0);
        // Dynamic formula: [16 [1 0] [1 42]] — contains hint opcode
        let hint_meta = Noun::cell(Noun::cell(Noun::Atom(1), Noun::Atom(0)), Noun::cell(Noun::Atom(1), Noun::Atom(0)));
        let body = Noun::cell(Noun::Atom(1), Noun::Atom(42));
        let dynamic = Noun::cell(Noun::Atom(16), Noun::cell(hint_meta, body));

        for _ in 0..5 {
            router.eval(&subj, &dynamic).unwrap();
        }
        // Dynamic formulas must not be memoized
        assert_eq!(router.stats().result_cache_entries, 0);
    }

    #[test]
    fn different_subjects_cached_separately() {
        let mut router = TierRouter::new(2);
        let formula = Noun::cell(Noun::Atom(1), Noun::Atom(99));

        // Same formula, two different subjects
        for _ in 0..3 {
            router.eval(&Noun::Atom(0), &formula).unwrap();
            router.eval(&Noun::Atom(1), &formula).unwrap();
        }

        // Each (formula, subject) pair cached separately
        assert_eq!(router.stats().result_cache_entries, 2);
    }
}
