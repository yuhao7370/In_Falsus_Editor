# Optimization Tracker

This document tracks executable optimization items for this round.

## Checklist

- [x] 1. Optimize hitsound triggering from full-scan-per-frame to cursor/binary-search.
- [x] 2. Optimize multi-drag updates to avoid O(k*n) `find` loops on every frame.
- [x] 3. Reduce per-frame allocations in minimap flick rendering.
- [x] 4. Optimize hitsound voice eviction (`Vec::remove(0)` -> queue-based).
- [x] 5. Reduce per-frame `track_path` cloning cost in frame snapshots.
- [x] 6. Add paste preview caching to avoid rebuilding preview notes every frame.

## Validation Rule

After each item:

1. Run `cargo test`.
2. If pass, commit once (no push).
