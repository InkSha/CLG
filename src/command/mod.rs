//! Command system.
//!
//! Commands operate on the **interface layer** (VFS), which in turn translates
//! to engine operations.  This follows the core design principle:
//!
//! > 命令作用于接口层 — Commands operate on the interface layer.
//!
//! ## Command hierarchy
//!
//! 1. **Basic** (入门): `ls`, `cd`, `pwd`
//! 2. **Info**: `cat`, `find`
//! 3. **Operations**: `cp`, `mv`, `rm`
//! 4. **Extended**: `attack`, `talk`, `use`
//! 5. **System**: `save`, `help`, `quit`

use crate::engine::area::FARM_AREA;
use crate::engine::combat::{run_combat, CombatResult};
use crate::engine::entity::EntityKind;
use crate::engine::World;
use crate::vfs::{Vfs, VfsEntry};

/// A parsed command.
#[derive(Debug)]
pub enum Command {
    // Basic (入门)
    Ls { path: Option<String> },
    Cd { path: String },
    Pwd,
    // Info
    Cat { file: String },
    Find { pattern: String },
    // Operations
    Cp { src: String, dst: String },
    Mv { src: String, dst: String },
    Rm { path: String },
    // Extended
    Attack { target: String },
    Talk { target: String },
    Use { item: String, target: Option<String> },
    // System
    Save,
    Help,
    Quit,
}

/// Result of command execution.
#[allow(dead_code)]
pub enum ExecResult {
    /// Normal output to display.
    Output(String),
    /// The game should quit.
    Quit,
    /// No output needed (already printed interactively).
    Done,
}

/// Parse a raw input string into a `Command`.
pub fn parse(input: &str) -> Result<Command, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err(String::new());
    }

    let mut parts = input.splitn(3, char::is_whitespace);
    let cmd = parts.next().unwrap_or("");
    let arg1 = parts.next().map(|s| s.trim().to_string());
    let arg2 = parts.next().map(|s| s.trim().to_string());

    match cmd {
        "ls" => Ok(Command::Ls { path: arg1 }),
        "cd" => Ok(Command::Cd {
            path: arg1.unwrap_or_else(|| "~".into()),
        }),
        "pwd" => Ok(Command::Pwd),
        "cat" => Ok(Command::Cat {
            file: arg1.ok_or("用法: cat <文件>")?,
        }),
        "find" | "grep" => Ok(Command::Find {
            pattern: arg1.ok_or("用法: find <关键词>")?,
        }),
        "cp" => Ok(Command::Cp {
            src: arg1.ok_or("用法: cp <源> <目标>")?,
            dst: arg2.ok_or("用法: cp <源> <目标>")?,
        }),
        "mv" => Ok(Command::Mv {
            src: arg1.ok_or("用法: mv <源> <目标>")?,
            dst: arg2.ok_or("用法: mv <源> <目标>")?,
        }),
        "rm" => Ok(Command::Rm {
            path: arg1.ok_or("用法: rm <文件>")?,
        }),
        "attack" => Ok(Command::Attack {
            target: arg1.ok_or("用法: attack <目标>")?,
        }),
        "talk" => Ok(Command::Talk {
            target: arg1.ok_or("用法: talk <目标>")?,
        }),
        "use" => Ok(Command::Use {
            item: arg1.ok_or("用法: use <物品> [目标]")?,
            target: arg2,
        }),
        "save" => Ok(Command::Save),
        "help" => Ok(Command::Help),
        "quit" | "exit" => Ok(Command::Quit),
        _ => Err(format!(
            "未知命令: {}（输入 help 查看帮助）",
            cmd
        )),
    }
}

/// Execute a command against the world and VFS.
pub fn execute(cmd: Command, world: &mut World, vfs: &mut Vfs) -> ExecResult {
    match cmd {
        Command::Ls { path } => exec_ls(world, vfs, path),
        Command::Cd { path } => exec_cd(world, vfs, &path),
        Command::Pwd => ExecResult::Output(vfs.pwd().to_string()),
        Command::Cat { file } => exec_cat(world, vfs, &file),
        Command::Find { pattern } => exec_find(world, vfs, &pattern),
        Command::Cp { src, dst } => exec_cp(world, vfs, &src, &dst),
        Command::Mv { src, dst } => exec_mv(world, vfs, &src, &dst),
        Command::Rm { path } => exec_rm(world, vfs, &path),
        Command::Attack { target } => exec_attack(world, vfs, &target),
        Command::Talk { target } => exec_talk(world, vfs, &target),
        Command::Use { item, target } => exec_use(world, vfs, &item, target.as_deref()),
        Command::Save => ExecResult::Output("（use save in game loop）".into()),
        Command::Help => {
            match vfs.cat(world, "/proc/help") {
                Ok(text) => ExecResult::Output(text),
                Err(e) => ExecResult::Output(e),
            }
        }
        Command::Quit => ExecResult::Quit,
    }
}

// ── Command implementations ──────────────────────────────────────────────────

fn exec_ls(world: &World, vfs: &Vfs, path: Option<String>) -> ExecResult {
    match vfs.ls(world, path.as_deref()) {
        Ok(entries) => {
            let mut out = String::new();
            for entry in &entries {
                match entry {
                    VfsEntry::Dir(name) => out.push_str(&format!("{}/  ", name)),
                    VfsEntry::File(name) => out.push_str(&format!("{}  ", name)),
                }
            }
            if out.is_empty() {
                out = "（空目录）".into();
            }
            ExecResult::Output(out)
        }
        Err(e) => ExecResult::Output(e),
    }
}

fn exec_cd(world: &mut World, vfs: &mut Vfs, path: &str) -> ExecResult {
    match vfs.cd(world, path) {
        Ok(new_path) => {
            // If we navigated into an area, also move the player.
            if let Some(area) = vfs.current_area_from_cwd() {
                if area != world.player_area {
                    match world.move_player(&area) {
                        Ok(()) => {
                            return ExecResult::Output(format!("移动到 {}", area));
                        }
                        Err(e) => {
                            // Revert VFS cwd.
                            let old_area = format!("/{}", world.player_area);
                            vfs.set_cwd(&old_area);
                            return ExecResult::Output(e);
                        }
                    }
                }
            }
            ExecResult::Output(new_path)
        }
        Err(e) => ExecResult::Output(e),
    }
}

fn exec_cat(world: &World, vfs: &Vfs, file: &str) -> ExecResult {
    match vfs.cat(world, file) {
        Ok(content) => ExecResult::Output(content),
        Err(e) => ExecResult::Output(e),
    }
}

fn exec_find(world: &World, vfs: &Vfs, pattern: &str) -> ExecResult {
    let results = vfs.find(world, pattern);
    if results.is_empty() {
        ExecResult::Output(format!("未找到匹配 \"{}\" 的结果。", pattern))
    } else {
        ExecResult::Output(results.join("\n"))
    }
}

fn exec_cp(world: &mut World, vfs: &Vfs, src: &str, dst_area: &str) -> ExecResult {
    // cp copies an item entity to another area.
    let src_path = vfs.resolve(src);
    let parts: Vec<&str> = src_path.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() != 2 {
        return ExecResult::Output("cp: 只能复制区域内的实体文件".into());
    }
    let area = parts[0];
    let name = parts[1];

    let entity_id = match world.find_entity_in_area(area, name) {
        Some(id) => id,
        None => return ExecResult::Output(format!("cp: {}: 未找到", src)),
    };

    let entity = match world.entity(entity_id) {
        Some(e) => e.clone(),
        None => return ExecResult::Output(format!("cp: {}: 未找到", src)),
    };

    if entity.kind != EntityKind::Item {
        return ExecResult::Output("cp: 只能复制物品类型的实体".into());
    }

    // Resolve destination area.
    let dst_resolved = vfs.resolve(dst_area);
    let dst_parts: Vec<&str> = dst_resolved.split('/').filter(|s| !s.is_empty()).collect();
    let target_area = if dst_parts.len() == 1 {
        dst_parts[0].to_string()
    } else {
        return ExecResult::Output("cp: 目标必须是一个区域目录".into());
    };

    let props: Vec<(&str, crate::engine::Value)> = entity
        .state
        .iter()
        .map(|(k, v)| (k.as_str(), v.clone()))
        .collect();
    world.spawn_entity(entity.kind.clone(), &entity.name, &target_area, props);
    ExecResult::Output(format!("已复制 {} 到 /{}/", entity.name, target_area))
}

fn exec_mv(world: &mut World, vfs: &Vfs, src: &str, dst_area: &str) -> ExecResult {
    // mv moves an entity to another area.
    let src_path = vfs.resolve(src);
    let parts: Vec<&str> = src_path.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() != 2 {
        return ExecResult::Output("mv: 只能移动区域内的实体文件".into());
    }
    let area = parts[0];
    let name = parts[1];

    let entity_id = match world.find_entity_in_area(area, name) {
        Some(id) => id,
        None => return ExecResult::Output(format!("mv: {}: 未找到", src)),
    };

    // Resolve destination area.
    let dst_resolved = vfs.resolve(dst_area);
    let dst_parts: Vec<&str> = dst_resolved.split('/').filter(|s| !s.is_empty()).collect();
    let target_area = if dst_parts.len() == 1 {
        dst_parts[0].to_string()
    } else {
        return ExecResult::Output("mv: 目标必须是一个区域目录".into());
    };

    if let Some(entity) = world.entity_mut(entity_id) {
        entity.area = target_area.clone();
        let name = entity.name.clone();
        ExecResult::Output(format!("已移动 {} 到 /{}/", name, target_area))
    } else {
        ExecResult::Output(format!("mv: {}: 未找到", src))
    }
}

fn exec_rm(world: &mut World, vfs: &Vfs, path: &str) -> ExecResult {
    let resolved = vfs.resolve(path);
    let parts: Vec<&str> = resolved.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() != 2 {
        return ExecResult::Output("rm: 只能删除区域内的实体文件".into());
    }
    let area = parts[0];
    let name = parts[1];

    if name == ".info" {
        return ExecResult::Output("rm: 不能删除 .info 文件".into());
    }

    let entity_id = match world.find_entity_in_area(area, name) {
        Some(id) => id,
        None => return ExecResult::Output(format!("rm: {}: 未找到", path)),
    };

    let entity_name = world
        .entity(entity_id)
        .map(|e| e.name.clone())
        .unwrap_or_default();
    world.remove_entity(entity_id);
    ExecResult::Output(format!("已删除 {}", entity_name))
}

fn exec_attack(world: &mut World, vfs: &Vfs, target: &str) -> ExecResult {
    let area = match vfs.current_area_from_cwd() {
        Some(a) => a,
        None => return ExecResult::Output("attack: 只能在区域目录中攻击".into()),
    };

    if area == FARM_AREA {
        return ExecResult::Output("attack: 不能在农场攻击".into());
    }

    let entity_id = match world.find_entity_in_area(&area, target) {
        Some(id) => id,
        None => return ExecResult::Output(format!("attack: 未找到目标 \"{}\"", target)),
    };

    // Check entity kind.
    let kind = match world.entity(entity_id) {
        Some(e) => e.kind.clone(),
        None => return ExecResult::Output("attack: 目标不存在".into()),
    };

    if kind != EntityKind::Monster {
        return ExecResult::Output("attack: 只能攻击怪物类型的实体".into());
    }

    // Run combat (mutates player and entity in place).
    let mut entity = world.entities.remove(&entity_id).unwrap();
    let result = run_combat(&mut world.player, &mut entity);

    match result {
        CombatResult::Victory { exp, gold } => {
            world.player.gold += gold;
            let leveled = world.player.gain_exp(exp);
            let mut msg = format!("胜利！获得 {} 经验和 {} 金币。", exp, gold);
            if leveled {
                msg.push_str(&format!(" 升级了！当前等级 {}！", world.player.level));
            }
            // Don't re-insert defeated entity.
            ExecResult::Output(msg)
        }
        CombatResult::Defeat => {
            world.player.hp = 1;
            // Entity survives.
            world.entities.insert(entity_id, entity);
            ExecResult::Output("你被击败了...生命值恢复至 1 点。".into())
        }
        CombatResult::Fled => {
            // Entity survives.
            world.entities.insert(entity_id, entity);
            ExecResult::Output("你安全逃脱了。".into())
        }
    }
}

fn exec_talk(world: &World, vfs: &Vfs, target: &str) -> ExecResult {
    let area = match vfs.current_area_from_cwd() {
        Some(a) => a,
        None => return ExecResult::Output("talk: 只能在区域目录中对话".into()),
    };

    let entity_id = match world.find_entity_in_area(&area, target) {
        Some(id) => id,
        None => return ExecResult::Output(format!("talk: 未找到 \"{}\"", target)),
    };

    let entity = match world.entity(entity_id) {
        Some(e) => e,
        None => return ExecResult::Output("talk: 目标不存在".into()),
    };

    if entity.kind != EntityKind::Npc {
        return ExecResult::Output(format!("{} 不是一个可以对话的NPC。", entity.name));
    }

    let dialogue = entity.get_str("dialogue");
    if dialogue.is_empty() {
        ExecResult::Output(format!("{}: ......", entity.name))
    } else {
        ExecResult::Output(format!("{}: 「{}」", entity.name, dialogue))
    }
}

fn exec_use(
    world: &mut World,
    vfs: &Vfs,
    item: &str,
    target: Option<&str>,
) -> ExecResult {
    let area = match vfs.current_area_from_cwd() {
        Some(a) => a,
        None => return ExecResult::Output("use: 只能在区域目录中使用物品".into()),
    };

    // Farm-specific: use <crop_type> <plot_N>
    if area == FARM_AREA {
        return exec_use_farm(world, item, target);
    }

    // Regular area: use an item for its effect.
    let entity_id = match world.find_entity_in_area(&area, item) {
        Some(id) => id,
        None => return ExecResult::Output(format!("use: 未找到 \"{}\"", item)),
    };

    let entity = match world.entity(entity_id) {
        Some(e) => e.clone(),
        None => return ExecResult::Output("use: 物品不存在".into()),
    };

    if entity.kind != EntityKind::Item {
        return ExecResult::Output(format!("{} 不是一个可以使用的物品。", entity.name));
    }

    // Default item behavior: sell for gold.
    let gold = entity.get_int("gold_value") as u32;
    world.player.gold += gold;
    world.remove_entity(entity_id);
    ExecResult::Output(format!("使用了 {}，获得 {} 金币。", entity.name, gold))
}

fn exec_use_farm(world: &mut World, item: &str, target: Option<&str>) -> ExecResult {
    // Harvest a plot: use plot_N
    if item.starts_with("plot_") {
        if let Some(idx_str) = item.strip_prefix("plot_") {
            if let Ok(idx) = idx_str.parse::<usize>() {
                return match world.farm.harvest(idx) {
                    Some(gold) => {
                        world.player.gold += gold;
                        ExecResult::Output(format!("收获成功！获得 {} 金币。", gold))
                    }
                    None => ExecResult::Output("作物尚未成熟，或地块为空。".into()),
                };
            }
        }
    }

    // Start/collect animal breeding: use <animal_name>
    if let Some(idx) = world.farm.animals.iter().position(|a| a.name == item) {
        let animal = &world.farm.animals[idx];
        if animal.breeding {
            if animal.is_ready() {
                return match world.farm.collect_animal(idx) {
                    Some(gold) => {
                        world.player.gold += gold;
                        ExecResult::Output(format!("收集 {} 成功！获得 {} 金币。", item, gold))
                    }
                    None => ExecResult::Output("动物还未准备好。".into()),
                };
            }
            return ExecResult::Output(format!("{} 正在繁殖中...", item));
        }
        return match world.farm.start_breeding(idx) {
            Ok(()) => ExecResult::Output(format!("{} 开始繁殖！", item)),
            Err(e) => ExecResult::Output(e),
        };
    }

    // Plant a crop: use <crop_type> <plot_N>
    if let Some(target) = target {
        if let Some(idx_str) = target.strip_prefix("plot_") {
            if let Ok(plot_idx) = idx_str.parse::<usize>() {
                let crop_type = world.crop_types.iter().find(|c| c.name == item).cloned();
                if let Some(ct) = crop_type {
                    return match world.farm.plant(plot_idx, &ct) {
                        Ok(()) => ExecResult::Output(format!(
                            "已种植 {} 到地块 {}。",
                            ct.name, plot_idx
                        )),
                        Err(e) => ExecResult::Output(e),
                    };
                }
                return ExecResult::Output(format!("未知作物类型: {}", item));
            }
        }
    }

    ExecResult::Output(format!(
        "use: 在农场中，使用方式：\n  use <作物名> <plot_N>  种植\n  use <plot_N>           收获\n  use <动物名>           繁殖/收集"
    ))
}
