use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};

use notify::{recommended_watcher, Event, EventKind, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};

use crate::farming::{Animal, Crop, Farm};
use crate::player::Player;

pub const WORLD_DIR: &str = "world";
pub const FARM_AREA: &str = "农场";
pub const PLAYER_FILE: &str = "player.json";
pub const DEFAULT_START_AREA: &str = "森林";

/// Events emitted by the background filesystem watcher.
#[derive(Debug, Clone)]
pub enum WorldEvent {
    /// The player file was moved (renamed) into a new area directory.
    PlayerMoved { to_area: String },
    /// A non-config entity file was created in an area.
    EntityCreated { area: String, filename: String },
    /// A non-config entity file was removed from an area.
    EntityRemoved { area: String, filename: String },
}

/// Filesystem-backed world manager.
///
/// Architecture:
/// - `world/<area>/`               → game map / region (directory)
/// - `world/<area>/<entity>.json`  → game entity (player, enemy, item…)
/// - Player location               → which area directory holds `player.json`
/// - Player movement               → `std::fs::rename` of `player.json`
/// - Background watcher            → `notify` crate drives real-time events
pub struct WorldManager {
    world_path: PathBuf,
    event_rx: mpsc::Receiver<WorldEvent>,
    // Kept alive so the watcher thread keeps running.
    _watcher: notify::RecommendedWatcher,
}

impl WorldManager {
    /// Create (or open) the world directory and start the filesystem watcher.
    pub fn new() -> Result<Self, String> {
        let world_path = PathBuf::from(WORLD_DIR);
        std::fs::create_dir_all(&world_path).map_err(|e| e.to_string())?;

        let (tx, rx) = mpsc::channel::<WorldEvent>();
        let world_path_clone = world_path.clone();

        let mut watcher = recommended_watcher(move |result: notify::Result<Event>| {
            let Ok(event) = result else { return };
            for evt in translate_notify_event(&event, &world_path_clone) {
                let _ = tx.send(evt);
            }
        })
        .map_err(|e| e.to_string())?;

        watcher
            .watch(&world_path, RecursiveMode::Recursive)
            .map_err(|e| e.to_string())?;

        Ok(WorldManager {
            world_path,
            event_rx: rx,
            _watcher: watcher,
        })
    }

    // ── Area directory management ─────────────────────────────────────────────

    /// Create each area's subdirectory and write its `area.json` metadata file.
    pub fn init_areas(&self, areas: &[crate::exploration::Area]) -> Result<(), String> {
        for area in areas {
            let dir = self.world_path.join(&area.name);
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let cfg_path = dir.join("area.json");
            if !cfg_path.exists() {
                let json = serde_json::to_string_pretty(area).map_err(|e| e.to_string())?;
                std::fs::write(&cfg_path, json).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    // ── Player file operations ────────────────────────────────────────────────

    /// Scan area directories and return the one that currently holds `player.json`.
    pub fn find_player_area(&self, area_names: &[&str]) -> Option<String> {
        for name in area_names {
            if self.world_path.join(name).join(PLAYER_FILE).exists() {
                return Some(name.to_string());
            }
        }
        None
    }

    /// Write the player entity to `world/<area>/player.json`.
    pub fn write_player(&self, player: &Player, area: &str) -> Result<(), String> {
        let path = self.world_path.join(area).join(PLAYER_FILE);
        let json = serde_json::to_string_pretty(player).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())
    }

    /// Read the player entity from `world/<area>/player.json`.
    pub fn read_player(&self, area: &str) -> Result<Player, String> {
        let path = self.world_path.join(area).join(PLAYER_FILE);
        let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&json).map_err(|e| e.to_string())
    }

    /// Move `player.json` from one area directory to another.
    ///
    /// This is what "player movement" means in the filesystem-driven
    /// architecture: an `std::fs::rename` observed by the watcher.
    pub fn move_player(&self, from_area: &str, to_area: &str) -> Result<(), String> {
        let from = self.world_path.join(from_area).join(PLAYER_FILE);
        let to = self.world_path.join(to_area).join(PLAYER_FILE);
        std::fs::rename(&from, &to).map_err(|e| e.to_string())
    }

    // ── Generic entity file operations ───────────────────────────────────────

    /// Write any serializable entity to `world/<area>/<filename>`.
    pub fn write_entity<T: Serialize>(
        &self,
        area: &str,
        filename: &str,
        entity: &T,
    ) -> Result<(), String> {
        let path = self.world_path.join(area).join(filename);
        let json = serde_json::to_string_pretty(entity).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())
    }

    /// Delete an entity file from `world/<area>/<filename>` (no-op if absent).
    pub fn remove_entity(&self, area: &str, filename: &str) -> Result<(), String> {
        let path = self.world_path.join(area).join(filename);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Return whether `world/<area>/<filename>` exists.
    pub fn entity_exists(&self, area: &str, filename: &str) -> bool {
        self.world_path.join(area).join(filename).exists()
    }

    // ── Farm filesystem helpers ───────────────────────────────────────────────

    /// Create `world/农场/` and write initial plot / animal files (if missing).
    pub fn init_farm(&self, farm: &Farm) -> Result<(), String> {
        let farm_dir = self.world_path.join(FARM_AREA);
        std::fs::create_dir_all(&farm_dir).map_err(|e| e.to_string())?;

        for (i, plot) in farm.plots.iter().enumerate() {
            let path = farm_dir.join(format!("plot_{}.json", i));
            if !path.exists() {
                let fs_plot = crop_to_fs_plot(plot.as_ref());
                let json = serde_json::to_string_pretty(&fs_plot).map_err(|e| e.to_string())?;
                std::fs::write(&path, json).map_err(|e| e.to_string())?;
            }
        }

        for animal in &farm.animals {
            let path = farm_dir.join(format!("{}.json", animal.name));
            if !path.exists() {
                let fs_animal = FsAnimal {
                    name: animal.name.clone(),
                    breed_time_secs: animal.breed_time_secs,
                    yield_gold: animal.yield_gold,
                    breeding: false,
                    breed_started_at_secs: None,
                };
                let json = serde_json::to_string_pretty(&fs_animal).map_err(|e| e.to_string())?;
                std::fs::write(&path, json).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    /// Overwrite all farm entity files with the current in-memory state.
    pub fn sync_farm(&self, farm: &Farm) -> Result<(), String> {
        let farm_dir = self.world_path.join(FARM_AREA);
        std::fs::create_dir_all(&farm_dir).map_err(|e| e.to_string())?;

        for (i, plot) in farm.plots.iter().enumerate() {
            let fs_plot = crop_to_fs_plot(plot.as_ref());
            let json = serde_json::to_string_pretty(&fs_plot).map_err(|e| e.to_string())?;
            std::fs::write(farm_dir.join(format!("plot_{}.json", i)), json)
                .map_err(|e| e.to_string())?;
        }

        for animal in &farm.animals {
            let fs_animal = FsAnimal {
                name: animal.name.clone(),
                breed_time_secs: animal.breed_time_secs,
                yield_gold: animal.yield_gold,
                breeding: animal.breeding,
                breed_started_at_secs: animal.breed_started_at_secs,
            };
            let json = serde_json::to_string_pretty(&fs_animal).map_err(|e| e.to_string())?;
            std::fs::write(farm_dir.join(format!("{}.json", animal.name)), json)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Load farm state from `world/农场/` files, falling back to template defaults.
    pub fn load_farm(&self, template: &Farm) -> Result<Farm, String> {
        let farm_dir = self.world_path.join(FARM_AREA);

        let mut plots = Vec::new();
        for i in 0..template.plots.len() {
            let path = farm_dir.join(format!("plot_{}.json", i));
            if path.exists() {
                let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
                let fs_plot: FsPlot = serde_json::from_str(&json).map_err(|e| e.to_string())?;
                plots.push(fs_plot_to_crop(fs_plot));
            } else {
                plots.push(None);
            }
        }

        let mut animals = Vec::new();
        for tmpl in &template.animals {
            let path = farm_dir.join(format!("{}.json", tmpl.name));
            if path.exists() {
                let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
                let fs_animal: FsAnimal =
                    serde_json::from_str(&json).map_err(|e| e.to_string())?;
                animals.push(Animal {
                    name: fs_animal.name,
                    breed_time_secs: fs_animal.breed_time_secs,
                    yield_gold: fs_animal.yield_gold,
                    breeding: fs_animal.breeding,
                    breed_started_at_secs: fs_animal.breed_started_at_secs,
                });
            } else {
                animals.push(tmpl.clone());
            }
        }

        Ok(Farm { plots, animals })
    }

    // ── Event polling ─────────────────────────────────────────────────────────

    /// Drain all pending filesystem events from the watcher channel.
    pub fn poll_events(&self) -> Vec<WorldEvent> {
        let mut out = Vec::new();
        while let Ok(e) = self.event_rx.try_recv() {
            out.push(e);
        }
        out
    }
}

// ── Filesystem entity representations ────────────────────────────────────────

/// A farm plot serialized to disk.
#[derive(Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum FsPlot {
    #[serde(rename = "empty")]
    Empty,
    #[serde(rename = "occupied")]
    Occupied {
        crop_name: String,
        grow_time_secs: u64,
        yield_gold: u32,
        /// Unix timestamp (seconds) when the crop was planted.
        planted_at_secs: u64,
    },
}

/// An animal serialized to disk.
#[derive(Serialize, Deserialize)]
pub struct FsAnimal {
    pub name: String,
    pub breed_time_secs: u64,
    pub yield_gold: u32,
    pub breeding: bool,
    /// Unix timestamp (seconds) when breeding was started, if any.
    pub breed_started_at_secs: Option<u64>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn crop_to_fs_plot(crop: Option<&Crop>) -> FsPlot {
    match crop {
        None => FsPlot::Empty,
        Some(c) => FsPlot::Occupied {
            crop_name: c.name.clone(),
            grow_time_secs: c.grow_time_secs,
            yield_gold: c.yield_gold,
            planted_at_secs: c.planted_at_secs.unwrap_or_else(current_unix_secs),
        },
    }
}

fn fs_plot_to_crop(fs: FsPlot) -> Option<Crop> {
    match fs {
        FsPlot::Empty => None,
        FsPlot::Occupied {
            crop_name,
            grow_time_secs,
            yield_gold,
            planted_at_secs,
        } => Some(Crop {
            name: crop_name,
            grow_time_secs,
            yield_gold,
            planted_at_secs: Some(planted_at_secs),
        }),
    }
}

/// Current Unix time in whole seconds.
pub fn current_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Translate a raw `notify` event into zero or more `WorldEvent`s.
fn translate_notify_event(event: &Event, world_path: &Path) -> Vec<WorldEvent> {
    let mut out = Vec::new();
    match &event.kind {
        // Atomic rename: both source and destination are known.
        EventKind::Modify(notify::event::ModifyKind::Name(
            notify::event::RenameMode::Both,
        )) if event.paths.len() >= 2 => {
            let from = &event.paths[0];
            let to = &event.paths[1];
            if to.file_name() == Some(OsStr::new(PLAYER_FILE)) {
                if let Some(to_area) = area_of(to, world_path) {
                    out.push(WorldEvent::PlayerMoved { to_area });
                }
            } else {
                if let (Some(fa), Some(fn_)) = (area_of(from, world_path), from.file_name()) {
                    let f = fn_.to_string_lossy().to_string();
                    if !is_internal_file(&f) {
                        out.push(WorldEvent::EntityRemoved { area: fa, filename: f });
                    }
                }
                if let (Some(ta), Some(fn_)) = (area_of(to, world_path), to.file_name()) {
                    let f = fn_.to_string_lossy().to_string();
                    if !is_internal_file(&f) {
                        out.push(WorldEvent::EntityCreated { area: ta, filename: f });
                    }
                }
            }
        }
        // Rename destination only (cross-filesystem or partial inotify event).
        EventKind::Modify(notify::event::ModifyKind::Name(
            notify::event::RenameMode::To,
        )) => {
            if let Some(path) = event.paths.first() {
                if path.file_name() == Some(OsStr::new(PLAYER_FILE)) {
                    if let Some(to_area) = area_of(path, world_path) {
                        out.push(WorldEvent::PlayerMoved { to_area });
                    }
                } else if let (Some(area), Some(fn_)) =
                    (area_of(path, world_path), path.file_name())
                {
                    let f = fn_.to_string_lossy().to_string();
                    if !is_internal_file(&f) {
                        out.push(WorldEvent::EntityCreated { area, filename: f });
                    }
                }
            }
        }
        // File created.
        EventKind::Create(_) => {
            if let Some(path) = event.paths.first() {
                if let (Some(area), Some(fn_)) = (area_of(path, world_path), path.file_name()) {
                    let f = fn_.to_string_lossy().to_string();
                    if !is_internal_file(&f) {
                        out.push(WorldEvent::EntityCreated { area, filename: f });
                    }
                }
            }
        }
        // File removed.
        EventKind::Remove(_) => {
            if let Some(path) = event.paths.first() {
                if let (Some(area), Some(fn_)) = (area_of(path, world_path), path.file_name()) {
                    let f = fn_.to_string_lossy().to_string();
                    if !is_internal_file(&f) {
                        out.push(WorldEvent::EntityRemoved { area, filename: f });
                    }
                }
            }
        }
        _ => {}
    }
    out
}

/// Extract the area name (first path component under `world_path`).
fn area_of(path: &Path, world_path: &Path) -> Option<String> {
    let rel = path.strip_prefix(world_path).ok()?;
    let component = rel.components().next()?;
    Some(component.as_os_str().to_string_lossy().to_string())
}

/// Return true for files that are internal config and should not surface as entity events.
fn is_internal_file(filename: &str) -> bool {
    filename == "area.json" || filename == PLAYER_FILE
}
