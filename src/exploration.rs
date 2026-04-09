use rand::Rng;
use serde::{Deserialize, Serialize};
use crate::combat::{Enemy, create_enemies_for_area};
use crate::player::Player;

#[derive(Serialize, Deserialize, Clone)]
pub struct Area {
    pub name: String,
    pub description: String,
    pub level_req: u32,
    pub enemy_level: u32,
    pub explore_cost_hp: i32,
}

pub fn get_areas() -> Vec<Area> {
    vec![
        Area {
            name: "Forest".to_string(),
            description: "A peaceful woodland. Good for beginners.".to_string(),
            level_req: 1,
            enemy_level: 1,
            explore_cost_hp: 0,
        },
        Area {
            name: "Dark Caves".to_string(),
            description: "Damp tunnels filled with danger.".to_string(),
            level_req: 3,
            enemy_level: 3,
            explore_cost_hp: 2,
        },
        Area {
            name: "Haunted Ruins".to_string(),
            description: "Ancient ruins haunted by restless spirits.".to_string(),
            level_req: 5,
            enemy_level: 5,
            explore_cost_hp: 5,
        },
        Area {
            name: "Volcanic Wastes".to_string(),
            description: "Scorched land near an active volcano.".to_string(),
            level_req: 8,
            enemy_level: 8,
            explore_cost_hp: 10,
        },
        Area {
            name: "Dragon's Peak".to_string(),
            description: "The summit where dragons nest. Extremely dangerous.".to_string(),
            level_req: 12,
            enemy_level: 12,
            explore_cost_hp: 15,
        },
    ]
}

pub enum ExploreResult {
    Enemy(Enemy),
    Gold(u32),
    Nothing,
    Item(String),
}

pub fn explore(player: &Player, area_idx: usize) -> Result<ExploreResult, String> {
    let areas = get_areas();
    let area = areas.get(area_idx).ok_or("Invalid area.")?;

    if player.level < area.level_req {
        return Err(format!(
            "You need to be level {} to explore {}.",
            area.level_req, area.name
        ));
    }

    let mut rng = rand::thread_rng();
    let roll: u32 = rng.gen_range(1..=100);

    let result = if roll <= 50 {
        // 50% chance enemy encounter
        let mut enemies = create_enemies_for_area(area.enemy_level);
        let idx = rng.gen_range(0..enemies.len());
        ExploreResult::Enemy(enemies.remove(idx))
    } else if roll <= 70 {
        // 20% chance find gold
        let gold = rng.gen_range(5..=30) * area.enemy_level;
        ExploreResult::Gold(gold)
    } else if roll <= 85 {
        // 15% chance find item
        let items = ["Old Sword", "Leather Boots", "Health Potion", "Shield Fragment", "Magic Scroll"];
        let item = items[rng.gen_range(0..items.len())].to_string();
        ExploreResult::Item(item)
    } else {
        // 15% nothing
        ExploreResult::Nothing
    };

    Ok(result)
}
