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
            name: "森林".to_string(),
            description: "宁静的树林，适合新手探索。".to_string(),
            level_req: 1,
            enemy_level: 1,
            explore_cost_hp: 0,
        },
        Area {
            name: "黑暗洞穴".to_string(),
            description: "潮湿而充满危险的地下洞穴。".to_string(),
            level_req: 3,
            enemy_level: 3,
            explore_cost_hp: 2,
        },
        Area {
            name: "鬼魂废墟".to_string(),
            description: "古老的废墟，充斥着不安的亡灵。".to_string(),
            level_req: 5,
            enemy_level: 5,
            explore_cost_hp: 5,
        },
        Area {
            name: "火山荒地".to_string(),
            description: "活火山附近被灼烧的焦土。".to_string(),
            level_req: 8,
            enemy_level: 8,
            explore_cost_hp: 10,
        },
        Area {
            name: "龙之巅峰".to_string(),
            description: "龙族栖息的山顶，极度危险。".to_string(),
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
    let area = areas.get(area_idx).ok_or("无效区域。")?;

    if player.level < area.level_req {
        return Err(format!(
            "你需要达到 {} 级才能探索 {}。",
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
        let items = ["旧剑", "皮靴", "生命药水", "盾牌碎片", "魔法卷轴"];
        let item = items[rng.gen_range(0..items.len())].to_string();
        ExploreResult::Item(item)
    } else {
        // 15% nothing
        ExploreResult::Nothing
    };

    Ok(result)
}
