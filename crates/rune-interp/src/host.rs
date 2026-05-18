//! Host call bridge for rune's escape hatch to external computation.
//! Opcode 16 with host tag dispatches here.

use rune_ast::Noun;

#[derive(Debug)]
pub struct HostError {
    pub message: String,
}

/// Call a host computation. Returns a Noun (the result as a tree).
pub fn call_host(target: u64, args: &Noun) -> Result<Noun, HostError> {
    // Stub implementation — real WASM/GPU bridge in M5b
    // For now: return args unchanged (identity host)
    let _ = target;
    Ok(args.clone())
}

/// WASM module host call stub.
pub fn wasm(_module_hash: u64, _fn_name: u64, args: &Noun) -> Result<Noun, HostError> {
    Ok(args.clone())
}

/// wGPU compute shader stub.
pub fn gpu(_shader_hash: u64, data: &Noun) -> Result<Noun, HostError> {
    Ok(data.clone())
}

/// ONNX inference stub.
pub fn infer(_model_hash: u64, input: &Noun) -> Result<Noun, HostError> {
    Ok(input.clone())
}
