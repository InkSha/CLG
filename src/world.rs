use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};

use notify::{recommended_watcher, Event, EventKind, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};

use crate::exploration::{default_areas, Area};
use crate::farming::{Animal, AnimalType, Crop, CropType, Farm};
use crate::player::Player;
use crate::template;

pub const WORLD_DIR: &str = "world";
pub const FARM_AREA: &str = "农场";
pub const PLAYER_FILE: &str = "player.yaml";
pub const ACTION_FILE: &str = "action.yaml";
pub const CONFIG_DIR: &str = "config";
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
    /// A template file was created or modified; path is relative to `world/`.
    TemplateChanged { path: String },
}

/// Filesystem-backed world manager.
///
/// Architecture:
/// - `world/<area>/`                → game map / region (directory)
/// - `world/<area>/<entity>.yaml`   → game entity (player, enemy, item…)
/// - Player location                → which area directory holds `player.yaml`
/// - Player movement                → `std::fs::rename` of `player.yaml`
/// - Background watcher             → `notify` crate drives real-time events
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

    // ── Config file management ────────────────────────────────────────────────

    /// Create the `world/config/` directory and write all default config files
    /// (areas, crops, animals, action.yaml) if they do not already exist.
    pub fn init_config(&self) -> Result<(), String> {
        let cfg_dir = self.world_path.join(CONFIG_DIR);
        std::fs::create_dir_all(&cfg_dir).map_err(|e| e.to_string())?;

        // areas.yaml
        let areas_path = cfg_dir.join("areas.yaml");
        if !areas_path.exists() {
            let yaml = serde_yaml::to_string(&default_areas()).map_err(|e| e.to_string())?;
            std::fs::write(&areas_path, yaml).map_err(|e| e.to_string())?;
        }

        // crops.yaml
        let crops_path = cfg_dir.join("crops.yaml");
        if !crops_path.exists() {
            let yaml =
                serde_yaml::to_string(&Farm::default_crop_types()).map_err(|e| e.to_string())?;
            std::fs::write(&crops_path, yaml).map_err(|e| e.to_string())?;
        }

        // animals.yaml
        let animals_path = cfg_dir.join("animals.yaml");
        if !animals_path.exists() {
            let yaml =
                serde_yaml::to_string(&Farm::default_animal_types()).map_err(|e| e.to_string())?;
            std::fs::write(&animals_path, yaml).map_err(|e| e.to_string())?;
        }

        // action.yaml (global)
        let action_path = self.world_path.join(ACTION_FILE);
        if !action_path.exists() {
            let default = crate::actions::ActionMap::default_map();
            let yaml = default.to_yaml()?;
            std::fs::write(&action_path, yaml).map_err(|e| e.to_string())?;
        }

        // ui.yaml – default UI template
        let ui_path = cfg_dir.join("ui.yaml");
        if !ui_path.exists() {
            std::fs::write(&ui_path, crate::ui_template::DEFAULT_UI_TEMPLATE)
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    /// Load the UI template from `world/config/ui.yaml`, falling back to the
    /// built-in default template when the file is absent or cannot be parsed.
    pub fn load_ui_template(&self) -> crate::ui_template::UiTemplate {
        let path = self.world_path.join(CONFIG_DIR).join("ui.yaml");
        if path.exists() {
            match crate::ui_template::load_ui_template(&path) {
                Ok(t) => return t,
                Err(e) => eprintln!("⚠️  无法加载 UI 模板：{}", e),
            }
        }
        // Parse the built-in default.
        serde_yaml::from_str(crate::ui_template::DEFAULT_UI_TEMPLATE)
            .unwrap_or_default()
    }

    /// Load all UI templates listed in `template.include` from `world/config/`.
    ///
    /// Each include name `"foo"` maps to `world/config/foo.yaml`.
    /// Templates that cannot be loaded are silently skipped.
    pub fn load_ui_includes(
        &self,
        template: &crate::ui_template::UiTemplate,
    ) -> std::collections::HashMap<String, crate::ui_template::UiTemplate> {
        let cfg_dir = self.world_path.join(CONFIG_DIR);
        let mut map = std::collections::HashMap::new();
        for name in &template.include {
            let path = cfg_dir.join(format!("{}.yaml", name));
            if let Ok(inc) = crate::ui_template::load_ui_template(&path) {
                map.insert(name.clone(), inc);
            }
        }
        map
    }

    /// Load areas from `world/config/areas.yaml`, falling back to built-in defaults.
    pub fn load_areas_config(&self) -> Vec<Area> {
        let path = self.world_path.join(CONFIG_DIR).join("areas.yaml");
        if path.exists() {
            if let Ok(yaml) = std::fs::read_to_string(&path) {
                if let Ok(areas) = serde_yaml::from_str::<Vec<Area>>(&yaml) {
                    if !areas.is_empty() {
                        return areas;
                    }
                }
            }
        }
        default_areas()
    }

    /// Load crop types from `world/config/crops.yaml`, falling back to defaults.
    pub fn load_crop_types(&self) -> Vec<CropType> {
        let path = self.world_path.join(CONFIG_DIR).join("crops.yaml");
        if path.exists() {
            if let Ok(yaml) = std::fs::read_to_string(&path) {
                if let Ok(types) = serde_yaml::from_str::<Vec<CropType>>(&yaml) {
                    if !types.is_empty() {
                        return types;
                    }
                }
            }
        }
        Farm::default_crop_types()
    }

    /// Load animal types from `world/config/animals.yaml`, falling back to defaults.
    pub fn load_animal_types(&self) -> Vec<AnimalType> {
        let path = self.world_path.join(CONFIG_DIR).join("animals.yaml");
        if path.exists() {
            if let Ok(yaml) = std::fs::read_to_string(&path) {
                if let Ok(types) = serde_yaml::from_str::<Vec<AnimalType>>(&yaml) {
                    if !types.is_empty() {
                        return types;
                    }
                }
            }
        }
        Farm::default_animal_types()
    }

    /// Build a default farm template using config-loaded animal types.
    pub fn make_default_farm(&self) -> Farm {
        let animal_types = self.load_animal_types();
        Farm::from_types(&animal_types, 4)
    }

    // ── Action map loading ────────────────────────────────────────────────────

    /// Load the effective action map for `area`.
    ///
    /// Starts from the global `world/action.yaml`, then merges any
    /// area-specific `world/<area>/action.yaml` on top (area keys win).
    pub fn load_action_map(&self, area: &str) -> crate::actions::ActionMap {
        let global_path = self.world_path.join(ACTION_FILE);
        let mut map = if global_path.exists() {
            crate::actions::ActionMap::load(&global_path)
                .unwrap_or_else(|_| crate::actions::ActionMap::default_map())
        } else {
            crate::actions::ActionMap::default_map()
        };

        let area_path = self.world_path.join(area).join(ACTION_FILE);
        if area_path.exists() {
            if let Ok(area_map) = crate::actions::ActionMap::load(&area_path) {
                map.merge(&area_map);
            }
        }

        map
    }

    // ── Entity listing / search helpers ──────────────────────────────────────

    /// List all files in `world/<area>/` (including internal config files).
    pub fn list_area_files(&self, area: &str) -> Vec<String> {
        let dir = self.world_path.join(area);
        let mut names: Vec<String> = std::fs::read_dir(&dir)
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        names.sort();
        names
    }

    /// Read the raw YAML content of an entity file in `world/<area>/<file>`.
    pub fn read_entity_raw(&self, area: &str, file: &str) -> Result<String, String> {
        let path = self.world_path.join(area).join(file);
        std::fs::read_to_string(&path).map_err(|e| e.to_string())
    }

    /// Write raw text content to `world/<area>/<file>`.
    pub fn write_entity_raw(&self, area: &str, file: &str, content: &str) -> Result<(), String> {
        let path = self.world_path.join(area).join(file);
        std::fs::write(&path, content).map_err(|e| e.to_string())
    }

    /// Search `world/<area>/` for files or content lines matching `pattern`.
    ///
    /// Returns a list of `(filename, matched_lines)` where `matched_lines` is
    /// a list of `(line_number, line_text)` pairs from the file content.
    /// Files whose *name* matches are included even if no content lines match.
    pub fn search_area(&self, area: &str, pattern: &str) -> Vec<(String, Vec<(usize, String)>)> {
        let pat = pattern.to_lowercase();
        let mut results = Vec::new();
        for filename in self.list_area_files(area) {
            let name_matches = filename.to_lowercase().contains(&pat);
            let mut line_matches = Vec::new();
            if let Ok(content) = self.read_entity_raw(area, &filename) {
                for (i, line) in content.lines().enumerate() {
                    if line.to_lowercase().contains(&pat) {
                        line_matches.push((i + 1, line.to_string()));
                    }
                }
            }
            if name_matches || !line_matches.is_empty() {
                results.push((filename, line_matches));
            }
        }
        results
    }

    // ── Area directory management ─────────────────────────────────────────────

    /// Create each area's subdirectory and write its `area.yaml` metadata file.
    pub fn init_areas(&self, areas: &[crate::exploration::Area]) -> Result<(), String> {
        for area in areas {
            let dir = self.world_path.join(&area.name);
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let cfg_path = dir.join("area.yaml");
            if !cfg_path.exists() {
                let yaml = serde_yaml::to_string(area).map_err(|e| e.to_string())?;
                std::fs::write(&cfg_path, yaml).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    // ── Player file operations ────────────────────────────────────────────────

    /// Scan area directories and return the one that currently holds `player.yaml`.
    pub fn find_player_area(&self, area_names: &[&str]) -> Option<String> {
        for name in area_names {
            if self.world_path.join(name).join(PLAYER_FILE).exists() {
                return Some(name.to_string());
            }
        }
        None
    }

    /// Write the player entity to `world/<area>/player.yaml`.
    pub fn write_player(&self, player: &Player, area: &str) -> Result<(), String> {
        let path = self.world_path.join(area).join(PLAYER_FILE);
        let yaml = serde_yaml::to_string(player).map_err(|e| e.to_string())?;
        std::fs::write(path, yaml).map_err(|e| e.to_string())
    }

    /// Read the player entity from `world/<area>/player.yaml`.
    pub fn read_player(&self, area: &str) -> Result<Player, String> {
        let path = self.world_path.join(area).join(PLAYER_FILE);
        let yaml = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_yaml::from_str(&yaml).map_err(|e| e.to_string())
    }

    /// Move `player.yaml` from one area directory to another.
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
        let yaml = serde_yaml::to_string(entity).map_err(|e| e.to_string())?;
        std::fs::write(path, yaml).map_err(|e| e.to_string())
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
            let path = farm_dir.join(format!("plot_{}.yaml", i));
            if !path.exists() {
                let fs_plot = crop_to_fs_plot(plot.as_ref());
                let yaml = serde_yaml::to_string(&fs_plot).map_err(|e| e.to_string())?;
                std::fs::write(&path, yaml).map_err(|e| e.to_string())?;
            }
        }

        for animal in &farm.animals {
            let path = farm_dir.join(format!("{}.yaml", animal.name));
            if !path.exists() {
                let fs_animal = FsAnimal {
                    name: animal.name.clone(),
                    breed_time_secs: animal.breed_time_secs,
                    yield_gold: animal.yield_gold,
                    breeding: false,
                    breed_started_at_secs: None,
                };
                let yaml = serde_yaml::to_string(&fs_animal).map_err(|e| e.to_string())?;
                std::fs::write(&path, yaml).map_err(|e| e.to_string())?;
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
            let yaml = serde_yaml::to_string(&fs_plot).map_err(|e| e.to_string())?;
            std::fs::write(farm_dir.join(format!("plot_{}.yaml", i)), yaml)
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
            let yaml = serde_yaml::to_string(&fs_animal).map_err(|e| e.to_string())?;
            std::fs::write(farm_dir.join(format!("{}.yaml", animal.name)), yaml)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Load farm state from `world/农场/` files, falling back to template defaults.
    pub fn load_farm(&self, template: &Farm) -> Result<Farm, String> {
        let farm_dir = self.world_path.join(FARM_AREA);

        let mut plots = Vec::new();
        for i in 0..template.plots.len() {
            let path = farm_dir.join(format!("plot_{}.yaml", i));
            if path.exists() {
                let yaml = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
                let fs_plot: FsPlot = serde_yaml::from_str(&yaml).map_err(|e| e.to_string())?;
                plots.push(fs_plot_to_crop(fs_plot));
            } else {
                plots.push(None);
            }
        }

        let mut animals = Vec::new();
        for tmpl in &template.animals {
            let path = farm_dir.join(format!("{}.yaml", tmpl.name));
            if path.exists() {
                let yaml = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
                let fs_animal: FsAnimal =
                    serde_yaml::from_str(&yaml).map_err(|e| e.to_string())?;
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

    // ── Template support ──────────────────────────────────────────────────────

    /// Write the built-in example template files to `world/templates/` (once).
    ///
    /// Each template file is only written when it does not already exist,
    /// so user edits are never overwritten.
    pub fn init_templates(&self) -> Result<(), String> {
        let dir = self.world_path.join("templates");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        for (filename, content) in template::builtin_templates() {
            let path = dir.join(filename);
            if !path.exists() {
                std::fs::write(&path, content).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    /// Scan `world/` recursively for `*.template.yaml` files and apply each
    /// one (generating its output entity file) if the output does not yet exist.
    ///
    /// Returns a list of `(template_path, output_path)` pairs for every file
    /// that was actually generated during this call.
    pub fn scan_and_apply_templates(&self) -> Vec<(String, String)> {
        let mut generated = Vec::new();
        for tmpl_path in template::find_templates(&self.world_path) {
            match template::load_template(&tmpl_path) {
                Ok(tmpl) => {
                    let tmpl_str = tmpl_path.to_string_lossy().to_string();
                    // Determine the output path and whether it already exists.
                    let dir = tmpl_path.parent();
                    let already_exists = dir
                        .map(|d| d.join(&tmpl.output).exists())
                        .unwrap_or(false);
                    match template::apply_template(&tmpl_path, &tmpl, false) {
                        Ok(out_path) => {
                            let out_str = out_path.to_string_lossy().to_string();
                            // Only report files that were newly generated in this call.
                            if !already_exists {
                                generated.push((tmpl_str, out_str));
                            }
                        }
                        Err(e) => eprintln!("警告：应用模板 {} 失败: {}", tmpl_path.display(), e),
                    }
                }
                Err(e) => eprintln!("警告：{}", e),
            }
        }
        generated
    }

    /// Re-apply a single template file (overwriting any existing output).
    ///
    /// Used when the watcher emits a [`WorldEvent::TemplateChanged`] event.
    pub fn reapply_template(&self, relative_path: &str) -> Result<String, String> {
        let full_path = self.world_path.join(relative_path);
        let tmpl = template::load_template(&full_path)?;
        let out = template::apply_template(&full_path, &tmpl, true)?;
        Ok(out.to_string_lossy().to_string())
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
            } else if let Some(fn_) = to.file_name().and_then(|n| n.to_str()) {
                if template::is_template_filename(fn_) {
                    if let Ok(rel) = to.strip_prefix(world_path) {
                        out.push(WorldEvent::TemplateChanged {
                            path: rel.to_string_lossy().to_string(),
                        });
                    }
                } else {
                    if let (Some(fa), Some(fn_from)) = (area_of(from, world_path), from.file_name()) {
                        let f = fn_from.to_string_lossy().to_string();
                        if !is_internal_file(&f) {
                            out.push(WorldEvent::EntityRemoved { area: fa, filename: f });
                        }
                    }
                    if let Some(ta) = area_of(to, world_path) {
                        let f = fn_.to_string();
                        if !is_internal_file(&f) {
                            out.push(WorldEvent::EntityCreated { area: ta, filename: f });
                        }
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
                } else if let Some(fn_) = path.file_name().and_then(|n| n.to_str()) {
                    if template::is_template_filename(fn_) {
                        if let Ok(rel) = path.strip_prefix(world_path) {
                            out.push(WorldEvent::TemplateChanged {
                                path: rel.to_string_lossy().to_string(),
                            });
                        }
                    } else if let Some(area) = area_of(path, world_path) {
                        let f = fn_.to_string();
                        if !is_internal_file(&f) {
                            out.push(WorldEvent::EntityCreated { area, filename: f });
                        }
                    }
                }
            }
        }
        // File created or modified (data change).
        EventKind::Create(_)
        | EventKind::Modify(notify::event::ModifyKind::Data(_)) => {
            if let Some(path) = event.paths.first() {
                if let Some(fn_) = path.file_name().and_then(|n| n.to_str()) {
                    if template::is_template_filename(fn_) || is_ui_config_file(path, world_path) {
                        if let Ok(rel) = path.strip_prefix(world_path) {
                            out.push(WorldEvent::TemplateChanged {
                                path: rel.to_string_lossy().to_string(),
                            });
                        }
                    } else if let Some(area) = area_of(path, world_path) {
                        let f = fn_.to_string();
                        if !is_internal_file(&f) {
                            out.push(WorldEvent::EntityCreated { area, filename: f });
                        }
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
/// Returns `None` if the path is inside a reserved directory (e.g. `config/`).
fn area_of(path: &Path, world_path: &Path) -> Option<String> {
    let rel = path.strip_prefix(world_path).ok()?;
    let component = rel.components().next()?;
    let name = component.as_os_str().to_string_lossy().to_string();
    // The config directory is not a game area.
    if name == CONFIG_DIR {
        return None;
    }
    Some(name)
}

/// Return true for files that are internal config and should not surface as entity events.
fn is_internal_file(filename: &str) -> bool {
    filename == "area.yaml"
        || filename == PLAYER_FILE
        || filename == ACTION_FILE
        || template::is_template_filename(filename)
}

/// Return true when `path` points to one of the UI config YAML files in
/// `world/config/` that the game reloads at runtime (e.g. `ui.yaml`).
fn is_ui_config_file(path: &Path, world_path: &Path) -> bool {
    let expected = world_path.join(CONFIG_DIR).join("ui.yaml");
    path == expected
}
