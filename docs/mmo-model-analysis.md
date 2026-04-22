# MMO Model Analysis

## Current Networking Scope

The current multiplayer implementation is a WASM-only relay in [src/network.rs](d:\Projects\new_game\src\network.rs) that synchronizes player position only. It does not synchronize authoritative gameplay state such as economy, inventory, quests, NPC state, time progression, or stat decay.

## Mechanic Categories

### Shared-world candidates

- Player movement and presence
- Global time and season progression
- Weather, crises, and festivals
- NPC location and availability

### Still single-player/local-only today

- Money, savings, debt, and rent
- Inventory and item consumption
- Housing upgrades and furnishings
- Skills, milestones, reputation, and friendships
- Quest generation and completion
- Stat decay and condition recovery

## MMO Gaps

### Authority

All meaningful progression is client-owned in local resources and save data. That model is incompatible with an MMO economy because it allows arbitrary local mutation and cannot arbitrate conflicts.

### Consistency

Players currently simulate their own time, events, and NPC interactions independently. In an MMO model, those systems need either a shared world clock or explicit sharding/instancing rules.

### Contention

The game has no model for two players acting on the same NPC, shop inventory, quest board, or world event at once.

## Recommended Migration Order

1. Promote world time, weather, festivals, and crises to server-owned state.
2. Promote player stats, economy, and inventory mutations to validated server-side actions.
3. Define ownership rules for NPC interactions, quests, and social progression.
4. Add replication rules for player-visible world state and late join snapshots.
5. Revisit crafting, trading, and player-to-player interaction once authority and persistence are in place.

## Immediate Follow-up Areas

- [src/resources.rs](d:\Projects\new_game\src\resources.rs): identify which resources must become server-owned.
- [src/save.rs](d:\Projects\new_game\src\save.rs): separate local presentation settings from canonical progression data.
- [src/systems/interaction.rs](d:\Projects\new_game\src\systems\interaction.rs): convert direct stat mutation into action requests.
- [src/systems/time.rs](d:\Projects\new_game\src\systems\time.rs): define server-driven world time semantics.
- [src/systems/npc.rs](d:\Projects\new_game\src\systems\npc.rs): decide whether NPCs are globally shared, instanced, or sharded.