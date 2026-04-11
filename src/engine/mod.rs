//! Layer 1: Engine Core (本体层)
//!
//! The engine manages the game world's true state. Entities, areas, the player,
//! combat, and farming all live here as in-memory data structures.
//! Nothing in this layer touches the filesystem—that is the job of the
//! persistence layer.

pub mod area;
pub mod combat;
pub mod entity;
pub mod farming;
pub mod player;
pub mod world;

pub use entity::Value;
pub use world::World;

use std::time::{SystemTime, UNIX_EPOCH};

/// Current Unix time in whole seconds.
pub fn current_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
