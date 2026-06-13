# Room Cell

A **Room cell** is the fundamental composable atom of the Grand Pattern architecture — a spatial unit that holds an embedding database, a "vibe" vector, surprise history, and a tick counter. Rooms are standalone but compose via the murmur protocol to form connected agent spaces.

## Why It Matters

Agent architectures need a primitive that represents *place* — not just a message channel, but a persistent, stateful context where perception meets prediction. The Room cell provides this. Each Room accumulates an embedding database of perceptions (what was observed) and predictions (what was expected), computing surprise at each tick. Rooms connect to each other through murmur summaries — compressed snapshots of vibe and surprise that propagate through the network. This design enables spatial reasoning: an agent can "be in" a Room, move between Rooms, and inherit the accumulated context of each space. The Room is to agent architectures what the cell is to biological systems — the minimum viable unit of autonomous computation.

## How It Works

### Core Data Model

Each Room maintains:

```rust
struct Room<const D: usize> {
    id: Uuid,
    name: String,
    vibe: [f64; 16],              // 16-dimensional mood/atmosphere vector
    perception_db: Vec<Embedding<D>>,  // observed embeddings
    prediction_db: Vec<Embedding<D>>,  // predicted embeddings
    surprise_history: Vec<f64>,        // surprise at each tick
    gc_threshold: usize,               // max DB size before GC
    tick_count: u64,                   // logical clock
}
```

### Surprise Computation

At each tick, the Room compares its latest perception against its prediction database. Surprise is the prediction error:

```
surprise(t) = ||perception_t - nearest_prediction||²
```

where `||·||²` is squared Euclidean distance in the D-dimensional embedding space. High surprise means the Room's predictions were wrong — something unexpected happened. Persistent high surprise triggers adaptation (updating the prediction model).

Complexity: O(P × D) per tick where P = prediction database size.

### Vibe Vector

The 16-dimensional `vibe` vector is an exponential moving average of recent perceptions:

```
vibe ← (1 - α) · vibe + α · perception
```

where α is a learning rate (typically 0.01). This gives each Room a persistent "atmosphere" that changes slowly even as individual perceptions are volatile.

### Murmur Protocol

Rooms broadcast compressed summaries via `MurmurSummary`:

```rust
struct MurmurSummary {
    vibe_snapshot: [f64; 16],
    surprise_avg: f64,
    tick: u64,
    room_count: usize,
}
```

Connected Rooms receive murmurs and update their own vibe based on neighbors — a gossip protocol for spatial mood propagation. This is O(1) per murmur message.

### Embedding Structure

Each embedding carries metadata:

```rust
struct Embedding<const D: usize> {
    data: [f64; D],
    timestamp: u64,
    source: String,
    surprise: f64,
}
```

The `surprise` field records how surprising this perception was when it arrived — creating a rich searchable history.

## Quick Start

```rust
use room_cell::{Room, Uuid, Embedding};

fn main() {
    let mut room: Room<384> = Room::new(Uuid::new(), "main-hall");

    // Add a perception
    let perception = Embedding {
        data: [0.0; 384],
        timestamp: 1,
        source: "camera".into(),
        surprise: 0.5,
    };
    room.perception_db.push(perception);

    println!("Room: {} (tick {})", room.name, room.tick_count);
    println!("Vibe: {:?}", &room.vibe[..4]);
}
```

```bash
cargo build
cargo test
```

## API

| Type | Field/Method | Description |
|------|-------------|-------------|
| `Room<D>` | `id: Uuid` | Unique identifier |
| `Room<D>` | `name: String` | Human-readable name |
| `Room<D>` | `vibe: [f64; 16]` | 16-dim atmosphere vector |
| `Room<D>` | `perception_db` | Observed embeddings |
| `Room<D>` | `prediction_db` | Predicted embeddings |
| `Room<D>` | `surprise_history` | Per-tick surprise values |
| `Room<D>` | `gc_threshold` | Max embeddings before GC |
| `MurmurSummary` | `vibe_snapshot, surprise_avg, tick` | Compressed gossip message |
| `Embedding<D>` | `data, timestamp, source, surprise` | Perception record |
| `Uuid` | `new()`, `Display` | Zero-dependency UUID v4 |

## Architecture Notes

Room Cell is the spatial primitive where γ (perception, construction of understanding) meets η (surprise, the gap between expectation and reality). Each Room's `perception_db` is the γ record — what was built/observed. The `surprise_history` is the η signal — where predictions failed. Their ratio drives C (competence): a Room with low surprise and rich perception is highly competent. The murmur protocol propagates competence signals across the spatial network. See [ARCHITECTURE.md](https://github.com/SuperInstance/SuperInstance/blob/main/ARCHITECTURE.md).

## References

1. Schroff, F., Kalenichenko, D., & Philbin, J. (2015). "FaceNet: A Unified Embedding for Face Recognition and Clustering." *CVPR*. — On the geometry of embedding spaces.
2. Demers, A., et al. (1987). "Epidemic Algorithms for Replicated Database Maintenance." *PODC*. — Gossip protocols, the basis of the murmur protocol.
3. Friston, K. (2010). "The free-energy principle: a unified brain theory?" *Nature Reviews Neuroscience*, 11, 127–138. — Surprise minimization as a principle of self-organizing systems.

## License

MIT
