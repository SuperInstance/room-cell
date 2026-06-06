# room-cell

The fundamental room cell — standalone but composable atom of the Grand Pattern architecture

## Overview

`room-cell` — the fundamental unit of the Grand Pattern architecture.

A Room is the atom. Every other tool (vibe, jepa, murmur, tick, signal) plugs into this.

## Architecture

This crate sits within the **five-layer Oxide Stack**:

| Layer | Crate | Role |
|-------|-------|------|
| 1 | open-parallel | Async runtime (tokio fork) |
| 2 | pincher | "Vector DB as runtime, LLM as compiler" |
| 3 | flux-core | Bytecode VM + A2A agent protocol |
| 4 | cuda-oxide | Flux→MIR→Pliron→NVVM→PTX compiler |
| 5 | cudaclaw | Persistent GPU kernels, warp consensus, SmartCRDT |

The key insight: **ternary values {-1, 0, +1} map directly to GPU compute**. They pack 16× denser than FP32, enable XNOR+popcount matmul, and conservation laws become compile-time checks.

## Stats

| Metric | Value |
|--------|-------|
| Tests | 16 |
| Lines of Code | 487 |
| Public API Surface | 15 items |
| License | MIT |

## Installation

```toml
[dependencies]
room-cell = "0.1.0"
```

## Usage

```rust
use room_cell::*;
// See src/lib.rs tests for complete working examples
```

### Key Types

```
- pub struct Uuid(pub [u8; 16]);
    pub fn new() -> Self {
- pub struct Embedding<const D: usize> {
- pub struct MurmurSummary {
- pub struct Room<const D: usize> {
    pub fn new(name: impl Into<String>) -> Self {
    pub fn perceive(&mut self, embedding: Embedding<D>) -> f64 {
    pub fn predict(&self) -> [f64; D] {
    pub fn surprise(actual: &[f64; D], predicted: &[f64; D]) -> f64 {
    pub fn record_surprise(&mut self, s: f64) {
```

## Design Philosophy

This crate uses **ternary algebra** (Z₃) where every value is {-1, 0, +1}:

- **+1** → positive signal (healthy, allocated, converged, ready)
- **0** → neutral (pending, balanced, monitoring, degraded)
- **-1** → negative signal (failed, free, diverged, overloaded)

This isn't arbitrary — ternary is the natural encoding for:
1. **BitNet b1.58** (Microsoft) — ternary neural networks at 60% less power
2. **GPU warp voting** — hardware ballot instructions return ternary consensus
3. **Conservation laws** — {-1, 0, +1} preserves quantity (what goes in must come out)

## Testing

```bash
git clone https://github.com/SuperInstance/room-cell.git
cd room-cell
cargo test
```

## License

MIT
