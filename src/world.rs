/// File-system-driven world model.
///
/// Directories  = game areas / maps
/// Files        = game entities (player, enemies, items, animals, crops)
/// Player move  = rename `player.json` between area directories
/// State watch  = inotify / FSEvents via the `notify` crate
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};

use crate::player::Player;

// ─── Area / entity constants ─────────────────────────────────────────────────

pub const AREA_NAMES: &[&str] = &["森林", "黑暗洞穴", "鬼魂废墟", "火山荒地", "龙之巅峰"];
pub const STARTING_AREA: &str = "森林";
pub const FARM_DIR: &str = "农场";

// ─── Serialisable entity types stored as individual JSON files ───────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct EnemyFile {
    pub id: u64,
    pub name: String,
    pub hp: i32,
    pub max_hp: i32,
    pub attack: i32,
    pub defense: i32,
    pub exp_reward: u32,
    pub gold_reward: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemFile {
    pub id: u64,
    pub name: String,
    pub value: u32,
}

/// Area metadata written to `<area>/metadata.json`.
#[derive(Debug, Serialize, Deserialize)]
pub struct AreaMeta {
    pub name: String,
    pub description: String,
    pub level_req: u32,
    pub enemy_level: u32,
    pub explore_cost_hp: i32,
}

// ─── Events emitted by the watcher ───────────────────────────────────────────

#[derive(Debug)]
pub enum WorldEvent {
    /// A file was created externally (path is relative to world_dir).
    FileCreated(PathBuf),
    /// A file was deleted externally.
    FileDeleted(PathBuf),
    /// A file was modified externally.
    FileModified(PathBuf),
    /// `player.json` was found in a different area than expected (external move).
    PlayerMovedExternally { to_area: String },
}

// ─── WorldManager ─────────────────────────────────────────────────────────────

pub struct WorldManager {
    /// Root of the world directory tree (e.g. `./world/`).
    pub world_dir: PathBuf,
    /// Keeps the watcher alive for the process lifetime.
    _watcher: Option<RecommendedWatcher>,
    /// Receives raw notify events from the background watcher thread.
    event_rx: mpsc::Receiver<notify::Result<Event>>,
    /// Monotonically increasing ID for new entity files.
    next_entity_id: u64,
    /// Paths we wrote ourselves — used to suppress spurious self-generated events.
    own_writes: HashSet<PathBuf>,
}

impl WorldManager {
    /// Creates the full directory tree and starts the file-system watcher.
    pub fn new() -> Result<Self, String> {
        let world_dir = PathBuf::from("world");

        // world/areas/<name>/
        for area in AREA_NAMES {
            let area_dir = world_dir.join("areas").join(area);
            fs::create_dir_all(&area_dir).map_err(|e| e.to_string())?;

            let meta_path = area_dir.join("metadata.json");
            if !meta_path.exists() {
                let meta = area_meta_for(area);
                let json = serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?;
                fs::write(&meta_path, json).map_err(|e| e.to_string())?;
            }
        }

        // world/farm/
        fs::create_dir_all(world_dir.join("farm")).map_err(|e| e.to_string())?;

        // Start the notify watcher.
        let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
        let mut watcher =
            RecommendedWatcher::new(tx, Config::default()).map_err(|e| e.to_string())?;
        watcher
            .watch(&world_dir, RecursiveMode::Recursive)
            .map_err(|e| e.to_string())?;

        Ok(WorldManager {
            world_dir,
            _watcher: Some(watcher),
            event_rx: rx,
            next_entity_id: 1,
            own_writes: HashSet::new(),
        })
    }

    // ── Path helpers ──────────────────────────────────────────────────────────

    pub fn area_dir(&self, area_name: &str) -> PathBuf {
        self.world_dir.join("areas").join(area_name)
    }

    pub fn farm_dir(&self) -> PathBuf {
        self.world_dir.join("farm")
    }

    // ── Player ────────────────────────────────────────────────────────────────

    /// Write the player entity file into `area_name`'s directory.
    pub fn write_player(&mut self, player: &Player, area_name: &str) -> Result<(), String> {
        let path = self.area_dir(area_name).join("player.json");
        let json = serde_json::to_string_pretty(player).map_err(|e| e.to_string())?;
        self.own_writes.insert(path.clone());
        fs::write(path, json).map_err(|e| e.to_string())
    }

    /// Scan all area directories to find where `player.json` currently lives.
    /// Returns `(Player, area_name)` or `None` if no player file exists yet.
    pub fn find_player(&self) -> Option<(Player, String)> {
        for area in AREA_NAMES {
            let path = self.area_dir(area).join("player.json");
            if let Ok(json) = fs::read_to_string(&path) {
                if let Ok(player) = serde_json::from_str::<Player>(&json) {
                    return Some((player, area.to_string()));
                }
            }
        }
        None
    }

    /// Move the player to `to_area` by writing the updated file there and
    /// removing the old file — this is the "player movement = file rename" step.
    pub fn move_player(
        &mut self,
        player: &Player,
        from_area: &str,
        to_area: &str,
    ) -> Result<(), String> {
        let to_path = self.area_dir(to_area).join("player.json");
        let json = serde_json::to_string_pretty(player).map_err(|e| e.to_string())?;
        self.own_writes.insert(to_path.clone());
        fs::write(&to_path, json).map_err(|e| e.to_string())?;

        let from_path = self.area_dir(from_area).join("player.json");
        if from_path.exists() {
            self.own_writes.insert(from_path.clone());
            fs::remove_file(from_path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    // ── Enemies ───────────────────────────────────────────────────────────────

    /// Materialise an enemy as a JSON file in the area directory.
    pub fn spawn_enemy(&mut self, area_name: &str, enemy: &crate::combat::Enemy) -> Result<u64, String> {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        let file = EnemyFile {
            id,
            name: enemy.name.clone(),
            hp: enemy.hp,
            max_hp: enemy.max_hp,
            attack: enemy.attack,
            defense: enemy.defense,
            exp_reward: enemy.exp_reward,
            gold_reward: enemy.gold_reward,
        };
        let path = self.area_dir(area_name).join(format!("enemy_{}.json", id));
        let json = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
        self.own_writes.insert(path.clone());
        fs::write(path, json).map_err(|e| e.to_string())?;
        Ok(id)
    }

    /// Delete an enemy file once it has been defeated.
    pub fn remove_enemy(&mut self, area_name: &str, enemy_id: u64) -> Result<(), String> {
        let path = self.area_dir(area_name).join(format!("enemy_{}.json", enemy_id));
        if path.exists() {
            self.own_writes.insert(path.clone());
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    // ── Items ─────────────────────────────────────────────────────────────────

    /// Write an item file into an area directory.
    pub fn spawn_item(&mut self, area_name: &str, item_name: &str, value: u32) -> Result<u64, String> {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        let file = ItemFile {
            id,
            name: item_name.to_string(),
            value,
        };
        let path = self.area_dir(area_name).join(format!("item_{}.json", id));
        let json = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
        self.own_writes.insert(path.clone());
        fs::write(path, json).map_err(|e| e.to_string())?;
        Ok(id)
    }

    /// Remove an item file after it has been picked up.
    pub fn remove_item(&mut self, area_name: &str, item_id: u64) -> Result<(), String> {
        let path = self.area_dir(area_name).join(format!("item_{}.json", item_id));
        if path.exists() {
            self.own_writes.insert(path.clone());
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    // ── Farm – animals ────────────────────────────────────────────────────────

    /// Persist one animal's state to `farm/animal_<name>.json`.
    pub fn write_animal(&mut self, animal: &crate::farming::Animal) -> Result<(), String> {
        let path = self.farm_dir().join(format!("animal_{}.json", animal.name));
        let json = serde_json::to_string_pretty(animal).map_err(|e| e.to_string())?;
        self.own_writes.insert(path.clone());
        fs::write(path, json).map_err(|e| e.to_string())
    }

    /// Load all animal files from the farm directory.
    pub fn read_animals(&self) -> Vec<crate::farming::Animal> {
        let mut animals = Vec::new();
        if let Ok(entries) = fs::read_dir(self.farm_dir()) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("animal_") && n.ends_with(".json"))
                    .unwrap_or(false)
                {
                    if let Ok(json) = fs::read_to_string(&path) {
                        if let Ok(animal) = serde_json::from_str::<crate::farming::Animal>(&json) {
                            animals.push(animal);
                        }
                    }
                }
            }
        }
        // Keep stable order matching the original game definition.
        animals.sort_by_key(|a| a.name.clone());
        animals
    }

    // ── Farm – crops ──────────────────────────────────────────────────────────

    /// Persist a crop at plot `idx` to `farm/crop_<idx>.json`.
    pub fn write_crop(&mut self, idx: usize, crop: &crate::farming::Crop) -> Result<(), String> {
        let path = self.farm_dir().join(format!("crop_{}.json", idx));
        let json = serde_json::to_string_pretty(crop).map_err(|e| e.to_string())?;
        self.own_writes.insert(path.clone());
        fs::write(path, json).map_err(|e| e.to_string())
    }

    /// Delete a crop file once it has been harvested.
    pub fn remove_crop(&mut self, idx: usize) -> Result<(), String> {
        let path = self.farm_dir().join(format!("crop_{}.json", idx));
        if path.exists() {
            self.own_writes.insert(path.clone());
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Load all crop files from the farm directory.
    /// Returns `(plot_index, Crop)` pairs.
    pub fn read_crops(&self) -> Vec<(usize, crate::farming::Crop)> {
        let mut crops = Vec::new();
        if let Ok(entries) = fs::read_dir(self.farm_dir()) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = match path.file_name().and_then(|n| n.to_str()) {
                    Some(n) => n.to_string(),
                    None => continue,
                };
                if name.starts_with("crop_") && name.ends_with(".json") {
                    if let Some(idx_str) = name
                        .strip_prefix("crop_")
                        .and_then(|s| s.strip_suffix(".json"))
                    {
                        if let Ok(idx) = idx_str.parse::<usize>() {
                            if let Ok(json) = fs::read_to_string(&path) {
                                if let Ok(crop) =
                                    serde_json::from_str::<crate::farming::Crop>(&json)
                                {
                                    crops.push((idx, crop));
                                }
                            }
                        }
                    }
                }
            }
        }
        crops
    }

    // ── Event polling ─────────────────────────────────────────────────────────

    /// Drain the watcher channel and return any externally generated events.
    /// Paths written by the game itself are filtered out.
    pub fn poll_events(&mut self) -> Vec<WorldEvent> {
        let mut events = Vec::new();

        while let Ok(result) = self.event_rx.try_recv() {
            let event = match result {
                Ok(e) => e,
                Err(_) => continue,
            };

            for path in &event.paths {
                // Skip paths that we wrote ourselves.
                if self.own_writes.remove(path) {
                    continue;
                }

                let canonical = path.clone();
                match &event.kind {
                    EventKind::Create(_) => {
                        // Check if this is an externally moved player.json.
                        if is_player_file(path) {
                            if let Some(area) = area_of_path(&self.world_dir, path) {
                                events.push(WorldEvent::PlayerMovedExternally { to_area: area });
                                continue;
                            }
                        }
                        events.push(WorldEvent::FileCreated(canonical));
                    }
                    EventKind::Remove(_) => {
                        events.push(WorldEvent::FileDeleted(canonical));
                    }
                    EventKind::Modify(_) => {
                        events.push(WorldEvent::FileModified(canonical));
                    }
                    _ => {}
                }
            }
        }

        events
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn is_player_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n == "player.json")
        .unwrap_or(false)
}

/// Extract the area name from a path like `world/areas/<area>/...`.
fn area_of_path(world_dir: &Path, path: &Path) -> Option<String> {
    let areas_dir = world_dir.join("areas");
    let rel = path.strip_prefix(&areas_dir).ok()?;
    rel.components().next().and_then(|c| {
        c.as_os_str().to_str().map(|s| s.to_string())
    })
}

fn area_meta_for(name: &str) -> AreaMeta {
    match name {
        "森林" => AreaMeta {
            name: name.to_string(),
            description: "宁静的树林，适合新手探索。".to_string(),
            level_req: 1,
            enemy_level: 1,
            explore_cost_hp: 0,
        },
        "黑暗洞穴" => AreaMeta {
            name: name.to_string(),
            description: "潮湿而充满危险的地下洞穴。".to_string(),
            level_req: 3,
            enemy_level: 3,
            explore_cost_hp: 2,
        },
        "鬼魂废墟" => AreaMeta {
            name: name.to_string(),
            description: "古老的废墟，充斥着不安的亡灵。".to_string(),
            level_req: 5,
            enemy_level: 5,
            explore_cost_hp: 5,
        },
        "火山荒地" => AreaMeta {
            name: name.to_string(),
            description: "活火山附近被灼烧的焦土。".to_string(),
            level_req: 8,
            enemy_level: 8,
            explore_cost_hp: 10,
        },
        "龙之巅峰" => AreaMeta {
            name: name.to_string(),
            description: "龙族栖息的山顶，极度危险。".to_string(),
            level_req: 12,
            enemy_level: 12,
            explore_cost_hp: 15,
        },
        _ => AreaMeta {
            name: name.to_string(),
            description: String::new(),
            level_req: 1,
            enemy_level: 1,
            explore_cost_hp: 0,
        },
    }
}
