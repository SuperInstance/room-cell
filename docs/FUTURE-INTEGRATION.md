# Future Integration: room-cell

## Current State
A Rust crate defining the fundamental room cell — a standalone but composable atom of the Grand Pattern architecture. Zero dependencies. The building block for room-based computation.

## Integration Opportunities

### With ternary-cell
room-cell IS the prototype for ternary-cell's core. The `tick()` cycle that room-cell defines becomes ternary-cell's 6-phase tick (predict → perceive → surprise → vibe → gc → conservation). The integration: refactor room-cell's tick into the six named phases, add the ternary value system ({-1, 0, +1}), and connect it to the TernaryMessenger for inter-cell signaling. room-cell provides the structural skeleton; ternary-cell provides the ternary semantics.

### With construct-core
room-cell maps to construct-core's Layer 1 (SyncConstruct). A room cell can load/unload skills, query its state, and interact with neighbors. The `no_std + alloc` constraint of Layer 1 means room-cell runs on embedded targets — each room cell IS a construct.

### With room-as-codespace
A room is a collection of room-cells, each representing one domain concept. In the Codespace, room-cells are the active entities — they tick, they communicate, they evolve. The Codespace provides the compute; room-cell provides the logic.

## Dormant Ideas Now Unlockable
room-cell was standalone with no ecosystem. Now ternary-cell, ternary-protocol, and ternary-registry provide the full ternary stack. room-cell becomes the concrete implementation: it imports ternary-cell traits, uses ternary-protocol for messaging, and registers with ternary-registry for discovery.

## Potential in Mature Systems
Every room in the fleet is a `Vec<RoomCell>`. Each cell ticks independently, communicates with neighbors via ternary-protocol messages, and can be migrated between hardware tiers via construct-core's layered traits. The room abstraction is just room-cell composition.

## Cross-Pollination Ideas
- **spreadsheet-cells**: The spreadsheet cell model IS the room-cell model — cells with values, formulas, and neighbors
- **capitaine-1**: The heartbeat concept from Capitaine maps to room-cell's tick cycle
- **polln**: Polln's Hive concept maps to a room-cell grid

## Dependencies for Next Steps
- Refactor tick into 6-phase cycle
- Add ternary value system
- Implement TernaryMessenger for inter-cell signaling
- Register with ternary-registry
