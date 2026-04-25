"""
Task catalog for the gamedev agent — new_game (Bevy 0.15 / Rust) edition.

Six dimensions x four focus areas = 24 tasks. Iteration order is
dimension-first (one of each dimension before repeating), so the queue
stays balanced across the criteria the user cares about: graphics, ui/ux,
content, gameplay depth, code quality, and security.
"""

from __future__ import annotations

# ---------------------------------------------------------------------------
# Dimension catalog
# ---------------------------------------------------------------------------

# Each entry is (focus_id, one-line scope hint). The scope hint is appended
# to the task prompt so the model knows what kind of edit is in scope. Keep
# focus_ids stable - they are persisted to state.json and used in branch
# names. The agent is allowed to inspect any file under the repo, but only
# the focus area's listed paths may be written.

DIMENSIONS: dict[str, list[tuple[str, str, list[str]]]] = {
    "graphics": [
        (
            "water-shimmer",
            "Animate water/pond tiles with a soft sine-driven color or alpha shimmer.",
            ["src/systems/visual.rs", "src/main.rs"],
        ),
        (
            "npc-shadows",
            "Add the same soft drop shadow under NPCs that the player has.",
            ["src/setup.rs"],
        ),
        (
            "weather-particle-polish",
            "Tighten weather particle density, alpha or motion to feel less bare.",
            ["src/systems/visual.rs"],
        ),
        (
            "per-state-camera-zoom",
            "Smoothly tighten camera zoom indoors and ease back out outdoors.",
            ["src/systems/player.rs", "src/systems/visual.rs"],
        ),
    ],
    "ui-ux": [
        (
            "hud-bar-icons",
            "Add small inline glyph indicators next to HUD stat bar labels.",
            ["src/systems/hud.rs", "src/setup.rs"],
        ),
        (
            "notification-easing",
            "Smooth the notification slide-in/out with an ease-out curve.",
            ["src/systems/hud.rs"],
        ),
        (
            "interactable-prompt-polish",
            "Improve the interactable highlight pulse and prompt readability.",
            ["src/systems/visual.rs", "src/systems/hud.rs"],
        ),
        (
            "skill-panel-bars",
            "Replace skill panel text with thin progress bars.",
            ["src/systems/hud.rs", "src/setup.rs"],
        ),
    ],
    "content": [
        (
            "npc-dialogue-variety",
            "Add at least four new context-aware NPC dialogue variants.",
            ["src/systems/npc.rs", "src/systems/interaction.rs"],
        ),
        (
            "seasonal-flavor-text",
            "Add season-aware flavor lines (weather, holidays, mood).",
            ["src/systems/hud.rs", "src/systems/interaction.rs"],
        ),
        (
            "new-quest-variant",
            "Add one new quest variant wired through the existing quest board.",
            ["src/systems/interaction.rs", "src/resources.rs"],
        ),
        (
            "restaurant-menu-expansion",
            "Add at least three new restaurant menu items with distinct effects.",
            ["src/systems/interaction.rs", "src/resources.rs"],
        ),
    ],
    "gameplay-depth": [
        (
            "skill-tier-thresholds",
            "Define skill tier thresholds and surface tier-up notifications.",
            ["src/resources.rs", "src/systems/interaction.rs", "src/systems/hud.rs"],
        ),
        (
            "weather-stat-effects",
            "Make weather meaningfully affect player energy/stress over time.",
            ["src/systems/player.rs", "src/resources.rs"],
        ),
        (
            "friendship-gift-event",
            "Add a friendship gift or birthday interaction with an NPC.",
            ["src/systems/interaction.rs", "src/systems/npc.rs"],
        ),
        (
            "crisis-recovery-path",
            "Add a Crisis-tier recovery action that nudges life rating upward.",
            ["src/systems/interaction.rs", "src/resources.rs"],
        ),
    ],
    "code-quality": [
        (
            "visual-module-split",
            "Split src/systems/visual.rs into focused submodules without behavior change.",
            ["src/systems/visual.rs", "src/systems/mod.rs", "src/main.rs"],
        ),
        (
            "magic-number-constants",
            "Extract repeated magic numbers in a system into named consts.",
            ["src/systems/visual.rs", "src/systems/player.rs"],
        ),
        (
            "doc-comment-pass",
            "Add /// doc comments to public components and resources lacking them.",
            ["src/components.rs", "src/resources.rs"],
        ),
        (
            "audio-system-tidy",
            "Tidy audio.rs: extract constants, document public functions.",
            ["src/audio.rs"],
        ),
    ],
    "security": [
        (
            "save-schema-validation",
            "Validate save.json field ranges on load; reject out-of-range values.",
            ["src/save.rs"],
        ),
        (
            "network-message-bounds",
            "Clamp inbound network message sizes and reject oversized payloads.",
            ["src/network.rs"],
        ),
        (
            "panic-elimination",
            "Replace .unwrap()/expect on user-influenced paths with graceful fallback.",
            ["src/save.rs", "src/network.rs"],
        ),
        (
            "input-sanitization",
            "Sanitize and length-cap text input fields used by the typing overlay.",
            ["src/systems/hud.rs", "src/systems/interaction.rs"],
        ),
    ],
}

# Stable iteration order for dimensions so the queue is deterministic.
DIMENSION_ORDER = [
    "graphics",
    "ui-ux",
    "content",
    "gameplay-depth",
    "code-quality",
    "security",
]


def build_task_queue() -> list[tuple[str, str, str, list[str]]]:
    """Return all (dimension, focus, scope_hint, allowed_paths) tuples in
    dimension-first interleaved order: round 1 picks one focus from each
    dimension, round 2 picks the next, and so on."""
    rounds = max(len(v) for v in DIMENSIONS.values())
    queue: list[tuple[str, str, str, list[str]]] = []
    for r in range(rounds):
        for dim in DIMENSION_ORDER:
            entries = DIMENSIONS[dim]
            if r < len(entries):
                focus, scope, paths = entries[r]
                queue.append((dim, focus, scope, paths))
    return queue


def pick_next_task(state: dict) -> tuple[str, str, str, list[str]] | None:
    """Return the first task not in state['completed'], or None if exhausted."""
    completed = {tuple(pair) for pair in state.get("completed", [])}
    for task in build_task_queue():
        dim, focus, _, _ = task
        if (dim, focus) not in completed:
            return task
    return None
