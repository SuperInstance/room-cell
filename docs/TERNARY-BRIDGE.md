# Ternary Bridge — Connecting room-cell to ternary-cell

> How room-cell's Python-style tick cycle maps to ternary-cell's Rust production implementation,
> and what each can learn from the other.

## 1. Architecture Overview

| Aspect | room-cell (`Room<D>`) | ternary-cell (`TernaryCell`) |
|--------|----------------------|------------------------------|
| Language | Rust (but conceptually Pythonic) | Rust (production, `#![forbid(unsafe_code)]`) |
| State space | Continuous D-dimensional embeddings | Discrete ternary {-1, 0, +1} |
| Prediction | JEPA moving average over last 5 embeddings | Inbox signal aggregation |
| Surprise | Cosine distance (0.0–2.0) | Absolute difference (0–2) |
| Vibe | 16-dim vector, clamped [-1, 1] | Energy integer, clamped [0, 20] |
| GC | Prune low-surprise entries (keep top N) | Clear inbox |
| Conservation | \|perceptions\| ≈ \|predictions\| | Energy bounds + apoptosis trigger |
| Topology | Single cell, no neighbors | CellGrid 2D, 4-connected neighbors |
| Lifecycle | Alive only | Active / Apoptotic / Dividing |
| Signaling | None (murmur gossip is passive) | TernaryMessenger active push to neighbors |
| Coordination | None | Tissue (grid-level run/converge/consensus) |

## 2. Tick Phase Mapping

Both systems implement the same 6-phase cycle: **predict → perceive → surprise → vibe → gc → conservation**.

### Phase 1: Predict

```rust
// room-cell: JEPA moving average
fn predict(&self) -> [f64; D] {
    // Average last JEPA_WINDOW (5) embeddings
    let count = self.perception_db.len().min(5);
    // ... weighted average over perception_db[start..]
}

// ternary-cell: Inbox aggregation
fn predict(&mut self) {
    let combined: i32 = self.inbox.iter().map(|m| m.to_ternary() as i32).sum();
    self.prediction = if combined > 0 { 1 } else if combined < 0 { -1 } else { self.ternary_value };
}
```

**Bridge insight:** room-cell predicts from internal memory (JEPA); ternary-cell predicts from external signals (inbox). A hybrid would use both: JEPA for temporal continuity *and* inbox for neighbor influence.

### Phase 2: Perceive

```rust
// room-cell: Ingest embedding, compute surprise against prediction
fn perceive(&mut self, embedding: Embedding<D>) -> f64 {
    let predicted = self.predict();
    let s = Self::surprise(&embedding.data, &predicted);
    self.record_surprise(s);
    self.perception_db.push(embedding);
    s
}

// ternary-cell: Update value from inbox signals
fn perceive(&mut self) {
    let combined: i32 = self.inbox.iter().map(|m| m.to_ternary() as i32).sum();
    if combined != 0 {
        self.ternary_value = combined.clamp(-1, 1) as i8;
    }
}
```

**Bridge insight:** room-cell's perceive is data-driven (ingests an embedding); ternary-cell's perceive is signal-driven. room-cell should add an inbox for neighbor signals; ternary-cell should accept external data beyond just neighbor votes.

### Phase 3: Surprise

```rust
// room-cell: Cosine distance (continuous, 0.0–2.0)
fn surprise(actual: &[f64; D], predicted: &[f64; D]) -> f64 {
    1.0 - cosine_similarity(actual, predicted)
}

// ternary-cell: Absolute prediction error (discrete, 0–2)
fn compute_surprise(&mut self) -> i32 {
    self.surprise = (self.ternary_value as i32 - self.prediction as i32).abs();
    self.surprise
}
```

**Bridge insight:** room-cell's cosine surprise is richer — it captures partial mismatches and directional error. ternary-cell's discrete surprise is coarse but fast (integer ops, no floating point). The cosine approach could be quantized for ternary cells.

### Phase 4: Vibe

```rust
// room-cell: Update 16-dim vibe vector from surprise delta
fn update_vibe(&mut self) {
    let delta_s = self.surprise_history[len-1] - self.surprise_history[len-2];
    for vi in 0..16 {
        let contribution = last[idx] * delta_s * LEARNING_RATE;
        self.vibe[vi] = (self.vibe[vi] + contribution).clamp(-1.0, 1.0);
    }
}

// ternary-cell: Adjust integer energy from surprise
fn vibe(&mut self) {
    self.energy -= self.surprise;
    if self.surprise == 0 { self.energy += 1; } // sync bonus
}
```

**Bridge insight:** room-cell's vibe is a high-dimensional emotional fingerprint; ternary-cell's vibe is a scalar survival resource. room-cell's approach captures *what changed* (directionality); ternary-cell's captures *how much* (magnitude). A unified vibe would have both: vector direction + scalar magnitude (energy).

### Phase 5: GC

```rust
// room-cell: Keep highest-surprise entries
fn gc(&mut self) {
    if self.perception_db.len() > self.gc_threshold {
        self.perception_db.sort_by(|a, b| b.surprise.cmp(&a.surprise));
        self.perception_db.truncate(self.gc_threshold);
    }
}

// ternary-cell: Clear inbox
fn gc(&mut self) {
    self.inbox.clear();
}
```

**Bridge insight:** room-cell's GC is intelligent (keeps the most informative memories); ternary-cell's GC is simplistic (dumps everything). ternary-cell should adopt surprise-ranked pruning — keep the most surprising signals, not discard all.

### Phase 6: Conservation

```rust
// room-cell: Balance check |perceptions| ≈ |predictions|
fn check_conservation(&self) -> bool {
    (self.perception_db.len() as f64 - self.prediction_db.len() as f64).abs() <= 1.0
}

// ternary-cell: Energy bounds + apoptosis
fn conservation(&mut self) {
    self.energy = self.energy.clamp(0, 20);
    if self.energy == 0 { self.state = CellState::Apoptotic; }
    self.tick_count += 1;
}
```

**Bridge insight:** room-cell's conservation is an information-balance law (what goes in ≈ what goes out); ternary-cell's conservation is an energy-balance law (stay alive or die). Both are conservation laws, but over different resources.

## 3. What room-cell Has (ternary-cell Should Adopt)

### 3.1 Continuous Embedding Memory
room-cell's `perception_db` and `prediction_db` give cells long-term memory. ternary-cell is stateless between ticks (only carries `ternary_value`, `energy`, `surprise`). Adding a memory would enable learning.

```rust
// Proposal for ternary-cell
pub struct TernaryCell {
    // ... existing fields ...
    perception_log: Vec<i8>,   // last N perceived values
    prediction_log: Vec<i8>,   // last N predictions
}
```

### 3.2 Rich Vibe Vector
The 16-dim vibe captures nuanced state. ternary-cell's single `energy` integer is coarse. A vibe vector would let cells express complex internal states.

### 3.3 Surprise-Ranked GC
ternary-cell should keep high-surprise signals and discard expected ones, not just clear everything.

### 3.4 Murmur/Gossip Protocol
room-cell's `MurmurSummary` compresses cell state for lightweight gossip. ternary-cell has no equivalent — cells only signal immediate neighbors. A gossip layer would enable long-range coordination.

### 3.5 JEPA Prediction (Temporal Smoothing)
room-cell's moving-average prediction provides temporal continuity. ternary-cell's prediction is reactive (just inbox sum). JEPA would smooth out noise.

## 4. What ternary-cell Has (room-cell Should Backport)

### 4.1 TernaryMessenger Signaling
room-cell has **no inter-cell communication**. Adding `TernaryMessenger`-style discrete signals would enable room-cells to coordinate.

### 4.2 CellGrid + Neighbor Topology
room-cell is a singleton. `CellGrid` with 4-connected neighbors provides spatial structure and local interaction.

### 4.3 Cell Lifecycle (Division + Apoptosis)
room-cell cells never die or reproduce. ternary-cell's `divide()` and apoptosis enable emergent population dynamics.

### 4.4 Tissue Coordinator
`Tissue::run()`, `Tissue::is_converged()`, and `Tissue::consensus()` provide grid-level operations that room-cell lacks entirely.

### 4.5 Energy System
ternary-cell's energy mechanic creates natural selection pressure. room-cell has no equivalent — cells exist forever regardless of fitness.

### 4.6 Signal Propagation
`CellGrid::propagate_signals()` collects all emissions then delivers to neighbors. room-cell has no signaling at all.

## 5. Integration Plan

### Phase 1: Add Signaling to room-cell (2 days)
- Import `TernaryMessenger` type into room-cell
- Add `inbox: Vec<TernaryMessenger>` to `Room`
- Add `receive()` and `emit()` methods
- Wire into `perceive()` phase

### Phase 2: Add Grid Topology to room-cell (3 days)
- Create `RoomGrid<D>` mirroring `CellGrid`
- Add 4-connected neighbor lookup
- Add `propagate_signals()`
- Add `tick_all()` with parallel tick

### Phase 3: Add Energy + Lifecycle to room-cell (2 days)
- Add `energy: i32` and `CellState` to `Room`
- Implement `divide()` for room-cells
- Implement apoptosis when vibe degrades
- Wire into `conservation()` phase

### Phase 4: Add Memory to ternary-cell (2 days)
- Add `perception_log: Vec<i8>` to `TernaryCell`
- Implement JEPA-style moving average prediction
- Upgrade `gc()` to surprise-ranked pruning
- Add `MurmurSummary` generation

### Phase 5: Unified Tissue API (3 days)
- Define `Cellular` trait covering both systems
- `Room<D>: Cellular` and `TernaryCell: Cellular`
- Shared `Tissue` coordinator works with both
- Cross-system grids (ternary cells + room cells side by side)

## 6. Code Sketch: Hybrid Cell

```rust
/// A cell that combines room-cell's rich state with ternary-cell's signaling.
pub struct HybridCell<const D: usize> {
    // Identity
    pub id: u64,
    pub name: String,
    pub state: CellState,
    pub generation: u32,

    // Ternary (from ternary-cell)
    pub ternary_value: i8,
    pub energy: i32,
    inbox: Vec<TernaryMessenger>,

    // Continuous (from room-cell)
    pub vibe: [f64; 16],
    perception_db: Vec<Embedding<D>>,
    prediction_db: Vec<Embedding<D>>,
    surprise_history: Vec<f64>,

    // Tick
    tick_count: u64,
    prediction: i8,
    accumulated_surprise: i32,
}

impl<const D: usize> HybridCell<D> {
    pub fn tick(&mut self, embedding: Option<Embedding<D>>) -> TickReport {
        // 1. Predict (JEPA + inbox)
        let continuous_pred = self.jepa_predict();
        self.ternary_predict();

        // 2. Perceive (data + signals)
        if let Some(emb) = embedding {
            let cos_surprise = Self::cosine_surprise(&emb.data, &continuous_pred);
            self.perception_db.push(emb);
            self.surprise_history.push(cos_surprise);
        }
        self.ternary_perceive();

        // 3. Surprise (both metrics)
        let ternary_surprise = self.compute_ternary_surprise();

        // 4. Vibe (vector + energy)
        self.update_vibe();
        self.adjust_energy(ternary_surprise);

        // 5. GC (intelligent)
        self.intelligent_gc();

        // 6. Conservation (energy + info balance)
        self.enforce_conservation();

        TickReport {
            tick: self.tick_count,
            surprise: ternary_surprise,
            energy: self.energy,
            alive: self.is_alive(),
        }
    }
}
```

## 7. Shared Trait Vision

```rust
pub trait Cellular {
    type Value;
    type Signal;

    fn predict(&mut self);
    fn perceive(&mut self, signal: Self::Signal);
    fn compute_surprise(&mut self) -> i32;
    fn update_vibe(&mut self);
    fn gc(&mut self);
    fn enforce_conservation(&mut self);
    fn tick(&mut self) -> i32;

    fn emit(&self) -> Self::Signal;
    fn is_alive(&self) -> bool;
}
```

## 8. Quick Reference: Function Mapping

| Phase | room-cell method | ternary-cell method | Unified name |
|-------|-----------------|--------------------|--------------|
| Predict | `Room::predict()` | `TernaryCell::predict()` | `predict()` |
| Perceive | `Room::perceive(emb)` | `TernaryCell::perceive()` | `perceive(signal)` |
| Surprise | `Room::surprise(&a, &b)` | `TernaryCell::compute_surprise()` | `compute_surprise()` |
| Vibe | `Room::update_vibe()` | `TernaryCell::vibe()` | `update_vibe()` |
| GC | `Room::gc()` | `TernaryCell::gc()` | `gc()` |
| Conservation | `Room::check_conservation()` | `TernaryCell::conservation()` | `enforce_conservation()` |
| Full tick | `Room::tick(emb)` | `TernaryCell::tick()` | `tick()` |
| Emit | — | `TernaryCell::emit()` | `emit()` |
| Receive | — | `TernaryCell::receive()` | `receive()` |
| Divide | — | `TernaryCell::divide()` | `divide()` |
| Murmur | `Room::murmur_summary()` | — | `summarize()` |
