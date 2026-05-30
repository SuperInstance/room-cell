//! `room-cell` — the fundamental unit of the Grand Pattern architecture.
//!
//! A Room is the atom. Every other tool (vibe, jepa, murmur, tick, signal) plugs into this.

use std::fmt;

// ---------------------------------------------------------------------------
// Minimal zero-dependency UUID (v4-like random)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Uuid(pub [u8; 16]);

impl Default for Uuid {
    fn default() -> Self {
        Self::new()
    }
}

impl Uuid {
    pub fn new() -> Self {
        // Simple LCG-based pseudo-random for zero-dep UUIDs.
        // Use a mix of address-space & counter for entropy.
        static mut SEED: u64 = 0x_AAAA_BBBB_CCCC_DDDD;
        fn next_u64() -> u64 {
            unsafe {
                SEED = SEED.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                SEED
            }
        }
        let a = next_u64();
        let b = next_u64();
        let mut bytes = [0u8; 16];
        bytes[0..8].copy_from_slice(&a.to_le_bytes());
        bytes[8..16].copy_from_slice(&b.to_le_bytes());
        // Set version 4 variant bits
        bytes[6] = (bytes[6] & 0x0F) | 0x40;
        bytes[8] = (bytes[8] & 0x3F) | 0x80;
        Uuid(bytes)
    }
}

impl fmt::Display for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let b = &self.0;
        write!(
            f,
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-\
             {:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            b[0], b[1], b[2], b[3],
            b[4], b[5],
            b[6], b[7],
            b[8], b[9],
            b[10], b[11], b[12], b[13], b[14], b[15],
        )
    }
}

impl fmt::Debug for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Uuid({})", self)
    }
}

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// A D-dimensional embedding with metadata.
#[derive(Clone, Debug)]
pub struct Embedding<const D: usize> {
    pub data: [f64; D],
    pub timestamp: u64,
    pub source: String,
    pub surprise: f64,
}

/// Compressed room summary for gossip (murmur protocol).
#[derive(Clone, Debug)]
pub struct MurmurSummary {
    pub vibe_snapshot: [f64; 16],
    pub surprise_avg: f64,
    pub tick: u64,
    pub room_count: usize,
}

/// The fundamental Room cell.
#[derive(Clone, Debug)]
pub struct Room<const D: usize> {
    pub id: Uuid,
    pub name: String,
    pub vibe: [f64; 16],
    pub perception_db: Vec<Embedding<D>>,
    pub prediction_db: Vec<Embedding<D>>,
    pub surprise_history: Vec<f64>,
    pub gc_threshold: usize,
    pub tick_count: u64,
    pub last_murmur: Option<MurmurSummary>,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const VIBE_DIMS: usize = 16;
const DEFAULT_GC_THRESHOLD: usize = 100;
const JEPA_WINDOW: usize = 5;
const LEARNING_RATE: f64 = 0.1;
const CONSERVATION_TOL: f64 = 1.0;

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl<const D: usize> Room<D> {
    /// Create a new empty room.
    pub fn new(name: impl Into<String>) -> Self {
        Room {
            id: Uuid::new(),
            name: name.into(),
            vibe: [0.0; VIBE_DIMS],
            perception_db: Vec::new(),
            prediction_db: Vec::new(),
            surprise_history: Vec::new(),
            gc_threshold: DEFAULT_GC_THRESHOLD,
            tick_count: 0,
            last_murmur: None,
        }
    }

    /// Add an embedding to the perception database and compute surprise
    /// against the JEPA prediction.
    pub fn perceive(&mut self, embedding: Embedding<D>) -> f64 {
        let predicted = self.predict();
        let s = Self::surprise(&embedding.data, &predicted);
        self.record_surprise(s);
        self.perception_db.push(embedding);
        s
    }

    /// Simple JEPA predictor: average of the last N embeddings.
    /// Returns a zero embedding if the perception database is empty.
    pub fn predict(&self) -> [f64; D] {
        let count = self.perception_db.len().min(JEPA_WINDOW);
        if count == 0 {
            return [0.0; D];
        }
        let mut result = [0.0; D];
        let start = self.perception_db.len() - count;
        for emb in &self.perception_db[start..] {
            for (i, v) in emb.data.iter().enumerate() {
                result[i] += v;
            }
        }
        let inv = 1.0 / count as f64;
        for v in result.iter_mut() {
            *v *= inv;
        }
        result
    }

    /// Cosine distance between two D-dimensional vectors.
    /// Returns 0.0 for identical (or parallel) vectors, up to 1.0 for orthogonal,
    /// and 2.0 for opposite. If either vector is zero-length, returns 1.0.
    pub fn surprise(actual: &[f64; D], predicted: &[f64; D]) -> f64 {
        let mut dot = 0.0f64;
        let mut norm_a = 0.0f64;
        let mut norm_p = 0.0f64;
        for i in 0..D {
            dot += actual[i] * predicted[i];
            norm_a += actual[i] * actual[i];
            norm_p += predicted[i] * predicted[i];
        }
        let denom = norm_a.sqrt() * norm_p.sqrt();
        if denom < 1e-12 {
            return 1.0; // undefined → max surprise
        }
        let cos_sim = dot / denom;
        1.0 - cos_sim // cosine distance
    }

    /// Record a surprise value into history.
    pub fn record_surprise(&mut self, s: f64) {
        self.surprise_history.push(s);
    }

    /// Update the 16-dimensional vibe vector using finite-difference surprise.
    /// Maps the D-dim embedding space to 16 vibe dimensions.
    pub fn update_vibe(&mut self) {
        let len = self.surprise_history.len();
        if len < 2 {
            return;
        }
        let delta_s = self.surprise_history[len - 1] - self.surprise_history[len - 2];

        // Map D-dim perception to 16 vibe dims via simple strided projection
        let last = match self.perception_db.last() {
            Some(e) => &e.data,
            None => return,
        };

        for vi in 0..VIBE_DIMS {
            // Sample from embedding with stride
            let idx = (vi * D) / VIBE_DIMS;
            let contribution = last[idx] * delta_s * LEARNING_RATE;
            self.vibe[vi] = (self.vibe[vi] + contribution).clamp(-1.0, 1.0);
        }
    }

    /// Garbage-collect: prune low-surprise entries, keeping the most surprising ones.
    pub fn gc(&mut self) {
        if self.perception_db.len() <= self.gc_threshold {
            return;
        }
        // Sort by surprise descending (keep highest surprise)
        self.perception_db.sort_by(|a, b| {
            b.surprise.partial_cmp(&a.surprise).unwrap_or(std::cmp::Ordering::Equal)
        });
        self.perception_db.truncate(self.gc_threshold);
    }

    /// Check conservation law: |Z_in| ≈ |Z_out| within tolerance.
    pub fn check_conservation(&self) -> bool {
        let diff = (self.perception_db.len() as f64 - self.prediction_db.len() as f64).abs();
        diff <= CONSERVATION_TOL
    }

    /// Generate a compressed murmur summary for gossip.
    pub fn murmur_summary(&mut self) -> MurmurSummary {
        let surprise_avg = if self.surprise_history.is_empty() {
            0.0
        } else {
            self.surprise_history.iter().sum::<f64>() / self.surprise_history.len() as f64
        };
        let summary = MurmurSummary {
            vibe_snapshot: self.vibe,
            surprise_avg,
            tick: self.tick_count,
            room_count: self.perception_db.len(),
        };
        self.last_murmur = Some(summary.clone());
        self.last_murmur.clone().unwrap()
    }

    /// Run a full tick cycle: predict → perceive (external) → surprise → update_vibe → gc → check_conservation.
    /// This version runs with a provided embedding.
    pub fn tick(&mut self, embedding: Embedding<D>) -> bool {
        // 1. Generate prediction
        let predicted = self.predict();
        // Store prediction
        let mut pred_emb = Embedding {
            data: predicted,
            timestamp: self.tick_count,
            source: "jepa".into(),
            surprise: 0.0,
        };
        // 2. Perceive — computes surprise internally
        let s = self.perceive(embedding);
        pred_emb.surprise = s;
        self.prediction_db.push(pred_emb);
        // 3. Update vibe
        self.update_vibe();
        // 4. GC
        self.gc();
        // 5. Check conservation
        self.tick_count += 1;
        self.check_conservation()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_embedding<const D: usize>(value: f64, ts: u64) -> Embedding<D> {
        Embedding {
            data: [value; D],
            timestamp: ts,
            source: "test".into(),
            surprise: 0.0,
        }
    }

    fn make_varied_embedding<const D: usize>(seed: f64, ts: u64) -> Embedding<D> {
        let mut data = [0.0; D];
        for i in 0..D {
            data[i] = (seed + i as f64).sin();
        }
        Embedding {
            data,
            timestamp: ts,
            source: "test".into(),
            surprise: 0.0,
        }
    }

    // 1. Create room
    #[test]
    fn test_create_room() {
        let room: Room<8> = Room::new("test-room");
        assert_eq!(room.name, "test-room");
        assert!(room.perception_db.is_empty());
        assert!(room.prediction_db.is_empty());
        assert!(room.surprise_history.is_empty());
        assert_eq!(room.tick_count, 0);
        assert!(room.last_murmur.is_none());
    }

    // 2. Perceive adds to db
    #[test]
    fn test_perceive_adds_to_db() {
        let mut room: Room<8> = Room::new("p");
        let emb = make_embedding(1.0, 1);
        room.perceive(emb);
        assert_eq!(room.perception_db.len(), 1);
        assert_eq!(room.surprise_history.len(), 1);
    }

    // 3. Predict returns reasonable embedding
    #[test]
    fn test_predict_returns_embedding() {
        let mut room: Room<8> = Room::new("p");
        // Empty → zero
        let pred = room.predict();
        assert_eq!(pred, [0.0; 8]);

        room.perceive(make_embedding(1.0, 1));
        let pred = room.predict();
        assert!(pred.iter().all(|&v| (v - 1.0).abs() < 1e-10));
    }

    // 4. Surprise is 0 for identical embeddings
    #[test]
    fn test_surprise_zero_for_identical() {
        let v = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let s = Room::<8>::surprise(&v, &v);
        assert!(s.abs() < 1e-10, "surprise for identical should be ~0, got {s}");
    }

    // 5. Surprise is high for opposite embeddings
    #[test]
    fn test_surprise_high_for_opposite() {
        let a: [f64; 8] = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let b: [f64; 8] = [-1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let s = Room::<8>::surprise(&a, &b);
        assert!(s > 1.5, "surprise for opposites should be ~2.0, got {s}");
    }

    // 6. Vibe updates with surprise
    #[test]
    fn test_vibe_updates() {
        let mut room: Room<8> = Room::new("v");
        let e1 = make_varied_embedding(1.0, 1);
        let e2 = make_varied_embedding(5.0, 2);
        room.perceive(e1);
        // First perceive → only 1 surprise, no update yet
        room.update_vibe();
        assert!(room.vibe.iter().all(|&v| v == 0.0)); // need 2 surprises

        room.perceive(e2);
        room.update_vibe();
        let changed = room.vibe.iter().any(|&v| v != 0.0);
        assert!(changed, "vibe should have changed after second perceive + update");
    }

    // 7. GC prunes low-surprise entries
    #[test]
    fn test_gc_prunes_low_surprise() {
        let mut room: Room<8> = Room::new("gc");
        room.gc_threshold = 3;
        for i in 0..10 {
            let mut emb = make_embedding(i as f64, i);
            emb.surprise = if i < 7 { 0.01 } else { 0.99 };
            room.perception_db.push(emb);
        }
        room.gc();
        assert!(room.perception_db.len() <= 3);
        // Should keep the high-surprise ones
        assert!(room.perception_db.iter().all(|e| e.surprise > 0.5));
    }

    // 8. GC keeps high-surprise entries
    #[test]
    fn test_gc_keeps_high_surprise() {
        let mut room: Room<8> = Room::new("gc");
        room.gc_threshold = 2;
        for i in 0..5 {
            let mut emb = make_embedding(i as f64, i);
            emb.surprise = i as f64 * 0.25; // 0.0, 0.25, 0.5, 0.75, 1.0
            room.perception_db.push(emb);
        }
        room.gc();
        assert_eq!(room.perception_db.len(), 2);
        assert!(room.perception_db[0].surprise >= room.perception_db[1].surprise);
    }

    // 9. Conservation law holds initially
    #[test]
    fn test_conservation_initial() {
        let room: Room<8> = Room::new("c");
        assert!(room.check_conservation(), "empty room should satisfy conservation");
    }

    // 10. Murmur summary compresses correctly
    #[test]
    fn test_murmur_summary() {
        let mut room: Room<8> = Room::new("m");
        room.perceive(make_embedding(1.0, 1));
        room.tick_count = 5;
        let summary = room.murmur_summary();
        assert_eq!(summary.tick, 5);
        assert_eq!(summary.room_count, 1);
        assert!(summary.surprise_avg >= 0.0);
        assert!(room.last_murmur.is_some());
    }

    // 11. Tick runs full cycle
    #[test]
    fn test_tick_full_cycle() {
        let mut room: Room<8> = Room::new("t");
        let emb = make_embedding(1.0, 1);
        let result = room.tick(emb);
        assert_eq!(room.tick_count, 1);
        assert_eq!(room.perception_db.len(), 1);
        assert_eq!(room.prediction_db.len(), 1);
        assert!(room.surprise_history.len() >= 1);
        // conservation should hold after tick (1 perception, 1 prediction)
        assert!(result);
    }

    // 12. Multiple ticks accumulate vibe change
    #[test]
    fn test_multiple_ticks() {
        let mut room: Room<8> = Room::new("mt");
        for i in 0..10 {
            let emb = make_varied_embedding(i as f64, i);
            room.tick(emb);
        }
        assert_eq!(room.tick_count, 10);
        let any_nonzero = room.vibe.iter().any(|&v| v != 0.0);
        assert!(any_nonzero, "vibe should be nonzero after 10 varied ticks");
    }

    // 13. Room with no data handles gracefully
    #[test]
    fn test_empty_room_graceful() {
        let room: Room<8> = Room::new("e");
        let pred = room.predict();
        assert_eq!(pred, [0.0; 8]);
        assert!(room.check_conservation());
        assert!(room.surprise_history.is_empty());
    }

    // 14. Embedding dimension is generic — D=8
    #[test]
    fn test_generic_dim_8() {
        let _room: Room<8> = Room::new("d8");
        let emb = make_embedding::<8>(1.0, 1);
        assert_eq!(emb.data.len(), 8);
    }

    // 14b. Embedding dimension is generic — D=32
    #[test]
    fn test_generic_dim_32() {
        let mut room: Room<32> = Room::new("d32");
        let emb = make_embedding::<32>(1.0, 1);
        room.perceive(emb);
        assert_eq!(room.perception_db[0].data.len(), 32);
    }

    // 15. Vibe dims are bounded [-1, 1]
    #[test]
    fn test_vibe_bounded() {
        let mut room: Room<8> = Room::new("vb");
        for i in 0..100 {
            let emb = make_varied_embedding(i as f64 * 0.1, i);
            room.tick(emb);
        }
        for &v in &room.vibe {
            assert!(v >= -1.0 && v <= 1.0, "vibe dim {v} out of bounds");
        }
    }
}
