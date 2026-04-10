//! Layer 2: Virtual File System (接口层)
//!
//! The VFS presents the game world as a Linux-style directory tree.
//! Commands operate on this interface layer, which translates requests into
//! engine operations.
//!
//! # Directory layout
//!
//! ```text
//! /                           Root — lists all areas + proc
//! ├── <area>/                 Area directory — one per game region
//! │   ├── .info               Area description (read-only)
//! │   └── <entity_name>       Entity interface file
//! ├── 农场/                   Farm area (special)
//! │   ├── .info               Farm info
//! │   ├── plot_0 … plot_N     Farm plots
//! │   └── <animal_name>       Animals
//! └── proc/                   /proc-style system views
//!     ├── status              Player status
//!     ├── areas               Area overview
//!     └── help                Command reference
//! ```
//!
//! Files are **not** stored on the real filesystem — they are generated
//! on-the-fly from the in-memory `World`.

use crate::engine::area::FARM_AREA;
use crate::engine::World;

/// A single directory entry returned by `ls`.
#[derive(Debug, Clone)]
pub enum VfsEntry {
    Dir(String),
    File(String),
}

impl VfsEntry {
    pub fn name(&self) -> &str {
        match self {
            VfsEntry::Dir(n) | VfsEntry::File(n) => n,
        }
    }
}

/// Virtual filesystem interface.
pub struct Vfs {
    /// Current working directory path (e.g. `"/"`, `"/森林"`, `"/proc"`).
    cwd: String,
}

impl Vfs {
    pub fn new(start_area: &str) -> Self {
        Vfs {
            cwd: format!("/{}", start_area),
        }
    }

    /// Return the current working directory.
    pub fn pwd(&self) -> &str {
        &self.cwd
    }

    /// Resolve a (possibly relative) path against the cwd.
    pub fn resolve(&self, path: &str) -> String {
        if path.starts_with('/') {
            // Absolute path.
            normalize(path)
        } else if path == ".." {
            parent(&self.cwd)
        } else if path == "." || path.is_empty() {
            self.cwd.clone()
        } else if path == "~" {
            // Home = root.
            "/".to_string()
        } else {
            let combined = if self.cwd == "/" {
                format!("/{}", path)
            } else {
                format!("{}/{}", self.cwd, path)
            };
            normalize(&combined)
        }
    }

    /// Change the working directory.
    ///
    /// Validates that the target is a valid directory in the world. Returns the
    /// new path on success.
    pub fn cd(&mut self, world: &World, path: &str) -> Result<String, String> {
        let target = self.resolve(path);

        if !self.is_directory(world, &target) {
            return Err(format!("cd: {}: 不是一个目录", path));
        }

        self.cwd = target;
        Ok(self.cwd.clone())
    }

    /// Force-set the cwd (used when the engine moves the player).
    pub fn set_cwd(&mut self, path: &str) {
        self.cwd = path.to_string();
    }

    /// List entries in a directory.
    pub fn ls(&self, world: &World, path: Option<&str>) -> Result<Vec<VfsEntry>, String> {
        let target = match path {
            Some(p) => self.resolve(p),
            None => self.cwd.clone(),
        };

        if target == "/" {
            // Root: list all areas + proc.
            let mut entries: Vec<VfsEntry> = world
                .area_names()
                .into_iter()
                .map(|n| VfsEntry::Dir(n.to_string()))
                .collect();
            entries.push(VfsEntry::Dir("proc".into()));
            entries.sort_by(|a, b| a.name().cmp(b.name()));
            return Ok(entries);
        }

        let parts = split_path(&target);

        // /proc
        if parts.first().map(|s| s.as_str()) == Some("proc") {
            if parts.len() == 1 {
                return Ok(vec![
                    VfsEntry::File("status".into()),
                    VfsEntry::File("areas".into()),
                    VfsEntry::File("help".into()),
                ]);
            }
            return Err(format!("ls: {}: 不是一个目录", target));
        }

        // /<area>
        if parts.len() == 1 {
            let area = &parts[0];
            if !self.area_exists(world, area) {
                return Err(format!("ls: {}: 区域不存在", area));
            }
            let mut entries = vec![VfsEntry::File(".info".into())];

            if area == FARM_AREA {
                // Farm: show plots and animals.
                for i in 0..world.farm.plots.len() {
                    entries.push(VfsEntry::File(format!("plot_{}", i)));
                }
                for animal in &world.farm.animals {
                    entries.push(VfsEntry::File(animal.name.clone()));
                }
            } else {
                // Regular area: show entities.
                for entity in world.entities_in_area(area) {
                    entries.push(VfsEntry::File(entity.name.clone()));
                }
            }
            return Ok(entries);
        }

        Err(format!("ls: {}: 不是一个目录", target))
    }

    /// Read the content of a file.
    pub fn cat(&self, world: &World, path: &str) -> Result<String, String> {
        let target = self.resolve(path);
        let parts = split_path(&target);

        if parts.is_empty() {
            return Err("cat: /: 是一个目录".into());
        }

        // /proc/<file>
        if parts[0] == "proc" {
            if parts.len() != 2 {
                return Err(format!("cat: {}: 无此文件", target));
            }
            return self.read_proc(world, &parts[1]);
        }

        if parts.len() != 2 {
            return Err(format!("cat: {}: 无此文件", target));
        }

        let area = &parts[0];
        let filename = &parts[1];

        if !self.area_exists(world, area) {
            return Err(format!("cat: {}: 区域不存在", area));
        }

        // .info
        if filename == ".info" {
            if area == FARM_AREA {
                return Ok("名称: 农场\n描述: 种植作物、饲养动物的地方。\n".into());
            }
            return world
                .find_area(area)
                .map(|a| a.to_display())
                .ok_or_else(|| format!("cat: {}/.info: 无此文件", area));
        }

        // Farm entities
        if area == FARM_AREA {
            return self.read_farm_entity(world, filename);
        }

        // Regular entity
        world
            .find_entity_in_area(area, filename)
            .and_then(|id| world.entity(id))
            .map(|e| e.to_display())
            .ok_or_else(|| format!("cat: {}/{}: 无此文件", area, filename))
    }

    /// Search all areas for entities matching a pattern.
    pub fn find(&self, world: &World, pattern: &str) -> Vec<String> {
        let lower = pattern.to_lowercase();
        let mut results = Vec::new();

        for area_name in world.area_names() {
            // Check area name.
            if area_name.to_lowercase().contains(&lower) {
                results.push(format!("/{}/", area_name));
            }

            if area_name == FARM_AREA {
                for (i, plot) in world.farm.plots.iter().enumerate() {
                    if let Some(crop) = plot {
                        if crop.name.to_lowercase().contains(&lower) {
                            results.push(format!("/{}/plot_{}", FARM_AREA, i));
                        }
                    }
                }
                for animal in &world.farm.animals {
                    if animal.name.to_lowercase().contains(&lower) {
                        results.push(format!("/{}/{}", FARM_AREA, animal.name));
                    }
                }
            } else {
                for entity in world.entities_in_area(area_name) {
                    if entity.name.to_lowercase().contains(&lower) {
                        results.push(format!("/{}/{}", area_name, entity.name));
                    }
                }
            }
        }

        results
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Check whether a path refers to a valid directory.
    fn is_directory(&self, world: &World, path: &str) -> bool {
        if path == "/" {
            return true;
        }
        let parts = split_path(path);
        if parts.len() == 1 {
            let name = &parts[0];
            if name == "proc" {
                return true;
            }
            return self.area_exists(world, name);
        }
        false
    }

    /// Check whether an area name is valid.
    fn area_exists(&self, world: &World, name: &str) -> bool {
        if name == FARM_AREA {
            return true;
        }
        world.find_area(name).is_some()
    }

    /// Read a /proc file.
    fn read_proc(&self, world: &World, name: &str) -> Result<String, String> {
        match name {
            "status" => Ok(format!(
                "=== 玩家状态 ===\n{}\n当前位置: {}",
                world.player.status_display(),
                world.player_area
            )),
            "areas" => {
                let mut out = String::from("=== 区域列表 ===\n");
                for area in &world.areas {
                    let marker = if area.name == world.player_area {
                        " ← 当前"
                    } else {
                        ""
                    };
                    out.push_str(&format!(
                        "  {} (Lv.{} 要求, 敌人Lv.{}){}\n",
                        area.name, area.level_req, area.enemy_level, marker
                    ));
                }
                out.push_str(&format!("  {} (农场)\n", FARM_AREA));
                Ok(out)
            }
            "help" => Ok(HELP_TEXT.to_string()),
            _ => Err(format!("cat: /proc/{}: 无此文件", name)),
        }
    }

    /// Read a farm-specific virtual file (plot or animal).
    fn read_farm_entity(&self, world: &World, filename: &str) -> Result<String, String> {
        // Try plot
        if let Some(idx_str) = filename.strip_prefix("plot_") {
            let idx: usize = idx_str.parse().map_err(|_| format!("cat: 无效的地块编号: {}", filename))?;
            if idx >= world.farm.plots.len() {
                return Err(format!("cat: 无此文件: {}", filename));
            }
            return Ok(match &world.farm.plots[idx] {
                None => format!("[作物] 地块 {}\n状态: 空地\n", idx),
                Some(crop) => {
                    let status = if crop.is_ready() { "可以收获！" } else { "生长中..." };
                    format!(
                        "[作物] 地块 {}\n作物: {}\n状态: {}\n产出: {} 金币\n",
                        idx, crop.name, status, crop.yield_gold
                    )
                }
            });
        }

        // Try animal
        if let Some(animal) = world.farm.animals.iter().find(|a| a.name == filename) {
            let status = if animal.breeding {
                if animal.is_ready() {
                    "可以收集！"
                } else {
                    "繁殖中..."
                }
            } else {
                "空闲"
            };
            return Ok(format!(
                "[动物] {}\n状态: {}\n繁殖时间: {}秒\n产出: {} 金币\n",
                animal.name, status, animal.breed_time_secs, animal.yield_gold
            ));
        }

        Err(format!("cat: {}/{}: 无此文件", FARM_AREA, filename))
    }

    /// Extract the current area from the cwd (if applicable).
    pub fn current_area_from_cwd(&self) -> Option<String> {
        let parts = split_path(&self.cwd);
        if parts.len() == 1 && parts[0] != "proc" {
            Some(parts[0].clone())
        } else {
            None
        }
    }
}

// ── Path utilities ───────────────────────────────────────────────────────────

/// Normalize a path: collapse `..`, remove trailing `/`, ensure leading `/`.
fn normalize(path: &str) -> String {
    let mut components: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                components.pop();
            }
            other => components.push(other),
        }
    }
    if components.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", components.join("/"))
    }
}

/// Get the parent of a path.
fn parent(path: &str) -> String {
    let parts = split_path(path);
    if parts.len() <= 1 {
        "/".to_string()
    } else {
        format!("/{}", parts[..parts.len() - 1].join("/"))
    }
}

/// Split a path into non-empty components (strips leading `/`).
fn split_path(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

// ── Help text ────────────────────────────────────────────────────────────────

const HELP_TEXT: &str = "\
=== 命令帮助 ===

基础命令（入门）:
  ls [路径]          列出目录内容
  cd <路径>          切换目录/区域
  pwd                显示当前路径

信息命令:
  cat <文件>         查看文件内容
  find <关键词>      搜索文件

操作命令:
  cp <源> <目标>     复制文件
  mv <源> <目标>     移动文件
  rm <文件>          删除文件

扩展行为:
  attack <目标>      攻击怪物
  talk <目标>        与NPC对话
  use <物品> [目标]  使用物品

系统:
  save               保存游戏
  help               显示此帮助
  quit               退出游戏

路径示例:
  /森林              绝对路径
  ..                 上级目录
  .info              当前目录下的文件
  /proc/status       查看玩家状态
";
