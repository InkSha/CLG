//! Area (spatial structure).
//!
//! An area is a named region of the game world.  In the virtual filesystem
//! each area maps to a directory.  Areas form a flat namespace under the
//! root `/` path (no nesting for now).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Area {
    pub name: String,
    pub description: String,
    pub level_req: u32,
    pub enemy_level: u32,
    pub explore_cost_hp: i32,
}

impl Area {
    /// Render area info as human-readable text (the `.info` file content).
    pub fn to_display(&self) -> String {
        format!(
            "名称: {}\n描述: {}\n等级要求: {}\n敌人等级: {}\n探索消耗HP: {}",
            self.name, self.description, self.level_req, self.enemy_level, self.explore_cost_hp
        )
    }
}

/// Default areas shipped with the game.
pub fn default_areas() -> Vec<Area> {
    vec![
        Area {
            name: "森林".into(),
            description: "宁静的树林，适合新手探索。".into(),
            level_req: 1,
            enemy_level: 1,
            explore_cost_hp: 0,
        },
        Area {
            name: "黑暗洞穴".into(),
            description: "潮湿而充满危险的地下洞穴。".into(),
            level_req: 3,
            enemy_level: 3,
            explore_cost_hp: 2,
        },
        Area {
            name: "鬼魂废墟".into(),
            description: "古老的废墟，充斥着不安的亡灵。".into(),
            level_req: 5,
            enemy_level: 5,
            explore_cost_hp: 5,
        },
        Area {
            name: "火山荒地".into(),
            description: "活火山附近被灼烧的焦土。".into(),
            level_req: 8,
            enemy_level: 8,
            explore_cost_hp: 10,
        },
        Area {
            name: "龙之巅峰".into(),
            description: "龙族栖息的山顶，极度危险。".into(),
            level_req: 12,
            enemy_level: 12,
            explore_cost_hp: 15,
        },
    ]
}

pub const FARM_AREA: &str = "农场";
pub const DEFAULT_START_AREA: &str = "森林";
