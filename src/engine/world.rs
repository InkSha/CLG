//! World engine — the core of the entity layer (本体层).
//!
//! `World` owns **all** in-memory game state: the player, every entity, every
//! area, and the farm.  The virtual filesystem and command system read from
//! and write to the `World`; it never touches the real filesystem itself.

use std::collections::HashMap;

use rand::Rng;

use super::area::{Area, FARM_AREA};
use super::combat::generate_monsters;
use super::entity::{Entity, EntityId, EntityKind, Value};
use super::farming::{AnimalType, CropType, Farm};
use super::player::Player;

/// The game world — single source of truth for all runtime state.
pub struct World {
    pub player: Player,
    pub player_area: String,
    pub areas: Vec<Area>,
    pub entities: HashMap<EntityId, Entity>,
    pub farm: Farm,
    pub crop_types: Vec<CropType>,
    next_id: EntityId,
}

impl World {
    /// Create a new world with the given configuration.
    pub fn new(
        player: Player,
        start_area: &str,
        areas: Vec<Area>,
        crop_types: Vec<CropType>,
        animal_types: Vec<AnimalType>,
    ) -> Self {
        let farm = Farm::from_types(&animal_types, 4);
        let mut world = World {
            player,
            player_area: start_area.to_string(),
            areas,
            entities: HashMap::new(),
            farm,
            crop_types,
            next_id: 1,
        };
        // Populate the start area with entities.
        world.populate_area(start_area);
        world
    }

    /// Restore a world from saved state.
    pub fn from_save(
        player: Player,
        player_area: String,
        areas: Vec<Area>,
        entities: HashMap<EntityId, Entity>,
        farm: Farm,
        crop_types: Vec<CropType>,
        next_id: EntityId,
    ) -> Self {
        World {
            player,
            player_area,
            areas,
            entities,
            farm,
            crop_types,
            next_id,
        }
    }

    // ── Entity management ─────────────────────────────────────────────────────

    /// Spawn a new entity and return its id.
    pub fn spawn_entity(
        &mut self,
        kind: EntityKind,
        name: &str,
        area: &str,
        props: Vec<(&str, Value)>,
    ) -> EntityId {
        let id = self.next_id;
        self.next_id += 1;
        let mut state = HashMap::new();
        for (k, v) in props {
            state.insert(k.to_string(), v);
        }
        self.entities.insert(
            id,
            Entity {
                id,
                kind,
                name: name.to_string(),
                area: area.to_string(),
                state,
            },
        );
        id
    }

    /// Remove an entity from the world.
    pub fn remove_entity(&mut self, id: EntityId) {
        self.entities.remove(&id);
    }

    /// All entities currently in the given area.
    pub fn entities_in_area(&self, area: &str) -> Vec<&Entity> {
        let mut v: Vec<&Entity> = self
            .entities
            .values()
            .filter(|e| e.area == area)
            .collect();
        v.sort_by_key(|e| &e.name);
        v
    }

    /// Find an entity by name in the given area (case-insensitive partial match).
    pub fn find_entity_in_area(&self, area: &str, name: &str) -> Option<EntityId> {
        let lower = name.to_lowercase();
        self.entities
            .values()
            .find(|e| e.area == area && e.name.to_lowercase().contains(&lower))
            .map(|e| e.id)
    }

    /// Get a mutable reference to an entity.
    pub fn entity_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }

    /// Get a reference to an entity.
    pub fn entity(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }

    // ── Area management ───────────────────────────────────────────────────────

    /// Find an area by name.
    pub fn find_area(&self, name: &str) -> Option<&Area> {
        self.areas.iter().find(|a| a.name == name)
    }

    /// List all area names (including the farm).
    pub fn area_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.areas.iter().map(|a| a.name.as_str()).collect();
        if !names.contains(&FARM_AREA) {
            names.push(FARM_AREA);
        }
        names
    }

    /// Move the player to a new area.
    ///
    /// Checks level requirements and populates the area with entities if
    /// it hasn't been visited yet.
    pub fn move_player(&mut self, target: &str) -> Result<(), String> {
        if target == FARM_AREA {
            self.player_area = target.to_string();
            return Ok(());
        }

        let area = self
            .find_area(target)
            .ok_or_else(|| format!("未知区域：{}", target))?
            .clone();

        if self.player.level < area.level_req {
            return Err(format!(
                "等级不足！需要 Lv.{} 才能进入 {}（当前 Lv.{}）",
                area.level_req, area.name, self.player.level
            ));
        }

        // Apply exploration HP cost.
        if area.explore_cost_hp > 0 && self.player.hp > area.explore_cost_hp {
            self.player.take_damage(area.explore_cost_hp);
        }

        self.player_area = target.to_string();

        // Populate if no entities currently exist in the area.
        let has_entities = self.entities.values().any(|e| e.area == target);
        if !has_entities {
            self.populate_area(target);
        }

        Ok(())
    }

    /// Expose next_id for persistence.
    pub fn next_id(&self) -> EntityId {
        self.next_id
    }

    /// Populate an area with random monsters and items.
    pub fn populate_area(&mut self, area_name: &str) {
        let area = match self.find_area(area_name) {
            Some(a) => a.clone(),
            None => return,
        };

        let mut rng = rand::thread_rng();

        // Spawn 2–3 random monsters from the area's level pool.
        let monsters = generate_monsters(area.enemy_level);
        let count = rng.gen_range(2..=3).min(monsters.len());
        let mut indices: Vec<usize> = (0..monsters.len()).collect();
        // Shuffle and take `count`.
        for i in (1..indices.len()).rev() {
            let j = rng.gen_range(0..=i);
            indices.swap(i, j);
        }
        for &idx in indices.iter().take(count) {
            let (name, stats) = &monsters[idx];
            let props: Vec<(&str, Value)> = stats
                .iter()
                .map(|(k, v)| (*k, Value::Int(*v)))
                .collect();
            self.spawn_entity(EntityKind::Monster, name, area_name, props);
        }

        // Possibly spawn an NPC.
        if rng.gen_bool(0.5) {
            let npc_names = ["老者", "旅行商人", "隐士"];
            let npc_dialogues = [
                "欢迎来到这片土地，冒险者。小心前方的怪物。",
                "我这里有好东西卖，不过你现在还买不起。",
                "真正的力量不在于攻击，而在于知道何时该撤退。",
            ];
            let idx = rng.gen_range(0..npc_names.len());
            self.spawn_entity(
                EntityKind::Npc,
                npc_names[idx],
                area_name,
                vec![("dialogue", Value::Str(npc_dialogues[idx].to_string()))],
            );
        }

        // Possibly spawn an item.
        if rng.gen_bool(0.4) {
            let items = [
                ("草药", 15),
                ("旧剑", 20),
                ("皮靴", 12),
                ("魔法卷轴", 25),
                ("盾牌碎片", 18),
            ];
            let (item_name, gold_val) = items[rng.gen_range(0..items.len())];
            self.spawn_entity(
                EntityKind::Item,
                item_name,
                area_name,
                vec![
                    ("gold_value", Value::Int(gold_val)),
                    (
                        "description",
                        Value::Str(format!("一个{}，价值{}金币。", item_name, gold_val)),
                    ),
                ],
            );
        }
    }
}
