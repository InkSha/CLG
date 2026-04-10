//! Layer 3: Persistence (持久化层)
//!
//! Handles saving and loading game state, plus reading configuration files.
//! The world engine and VFS never touch the real filesystem; this layer is
//! the sole bridge between in-memory state and disk.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::engine::area::Area;
use crate::engine::entity::{Entity, EntityId};
use crate::engine::farming::{AnimalType, CropType, Farm};
use crate::engine::player::Player;
use crate::engine::World;

pub const SAVE_DIR: &str = "world";
const CONFIG_DIR: &str = "config";
const SAVE_FILE: &str = "save.yaml";

/// A complete snapshot of the game world, serializable to disk.
#[derive(Serialize, Deserialize)]
pub struct SaveData {
    pub player: Player,
    pub player_area: String,
    pub entities: HashMap<EntityId, Entity>,
    pub farm: Farm,
    pub next_id: EntityId,
}

// ── Config loading ───────────────────────────────────────────────────────────

/// Ensure the `world/config/` directory exists and write default config files
/// if they are absent.
pub fn init_config() {
    let cfg_dir = PathBuf::from(SAVE_DIR).join(CONFIG_DIR);
    let _ = std::fs::create_dir_all(&cfg_dir);

    write_default(&cfg_dir.join("areas.yaml"), || {
        serde_yaml::to_string(&crate::engine::area::default_areas()).ok()
    });

    write_default(&cfg_dir.join("crops.yaml"), || {
        serde_yaml::to_string(&Farm::default_crop_types()).ok()
    });

    write_default(&cfg_dir.join("animals.yaml"), || {
        serde_yaml::to_string(&Farm::default_animal_types()).ok()
    });
}

/// Load area definitions from `world/config/areas.yaml`.
pub fn load_areas() -> Vec<Area> {
    let path = PathBuf::from(SAVE_DIR)
        .join(CONFIG_DIR)
        .join("areas.yaml");
    load_yaml_or(&path, crate::engine::area::default_areas)
}

/// Load crop types from `world/config/crops.yaml`.
pub fn load_crop_types() -> Vec<CropType> {
    let path = PathBuf::from(SAVE_DIR)
        .join(CONFIG_DIR)
        .join("crops.yaml");
    load_yaml_or(&path, Farm::default_crop_types)
}

/// Load animal types from `world/config/animals.yaml`.
pub fn load_animal_types() -> Vec<AnimalType> {
    let path = PathBuf::from(SAVE_DIR)
        .join(CONFIG_DIR)
        .join("animals.yaml");
    load_yaml_or(&path, Farm::default_animal_types)
}

// ── Save / Load ──────────────────────────────────────────────────────────────

/// Save the current world state to `world/save.yaml`.
pub fn save_game(world: &World) -> Result<(), String> {
    let save_dir = PathBuf::from(SAVE_DIR);
    std::fs::create_dir_all(&save_dir).map_err(|e| e.to_string())?;

    let data = SaveData {
        player: world.player.clone(),
        player_area: world.player_area.clone(),
        entities: world.entities.clone(),
        farm: world.farm.clone(),
        next_id: world.next_id(),
    };

    let yaml = serde_yaml::to_string(&data).map_err(|e| e.to_string())?;
    std::fs::write(save_dir.join(SAVE_FILE), yaml).map_err(|e| e.to_string())
}

/// Load a saved game from `world/save.yaml`.
///
/// Returns `None` if no save file exists.
pub fn load_game(areas: &[Area], crop_types: &[CropType]) -> Option<World> {
    let path = PathBuf::from(SAVE_DIR).join(SAVE_FILE);
    if !path.exists() {
        return None;
    }
    let yaml = std::fs::read_to_string(&path).ok()?;
    let data: SaveData = serde_yaml::from_str(&yaml).ok()?;

    Some(World::from_save(
        data.player,
        data.player_area,
        areas.to_vec(),
        data.entities,
        data.farm,
        crop_types.to_vec(),
        data.next_id,
    ))
}

/// Check if a save file exists.
#[allow(dead_code)]
pub fn save_exists() -> bool {
    PathBuf::from(SAVE_DIR).join(SAVE_FILE).exists()
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn write_default<F>(path: &Path, generator: F)
where
    F: FnOnce() -> Option<String>,
{
    if !path.exists() {
        if let Some(content) = generator() {
            let _ = std::fs::write(path, content);
        }
    }
}

fn load_yaml_or<T, F>(path: &Path, fallback: F) -> Vec<T>
where
    T: serde::de::DeserializeOwned,
    F: FnOnce() -> Vec<T>,
{
    if path.exists() {
        if let Ok(yaml) = std::fs::read_to_string(path) {
            if let Ok(items) = serde_yaml::from_str::<Vec<T>>(&yaml) {
                if !items.is_empty() {
                    return items;
                }
            }
        }
    }
    fallback()
}
