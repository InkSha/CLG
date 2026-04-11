//! Turn-based combat system.
//!
//! Combat operates on the **engine layer**: it reads and writes player and
//! entity state directly.  UI calls go through `crate::ui`.

use rand::Rng;

use super::entity::Entity;
use super::player::Player;

/// Outcome of a combat encounter.
pub enum CombatResult {
    Victory { exp: u32, gold: u32 },
    Defeat,
    Fled,
}

fn calc_damage(attacker_atk: i32, defender_def: i32) -> i32 {
    let mut rng = rand::thread_rng();
    let base = (attacker_atk - defender_def).max(1);
    let variation = rng.gen_range(0.8_f64..=1.2_f64);
    ((base as f64 * variation).round() as i32).max(1)
}

/// Run an interactive combat encounter between the player and a monster entity.
///
/// The entity's HP is mutated in place during combat, so the caller can
/// inspect or remove it afterwards.
pub fn run_combat(player: &mut Player, entity: &mut Entity) -> CombatResult {
    let mut enemy_hp = entity.get_int("hp") as i32;
    let enemy_max_hp = entity.get_int("max_hp") as i32;
    let enemy_atk = entity.get_int("attack") as i32;
    let enemy_def = entity.get_int("defense") as i32;
    let exp_reward = entity.get_int("exp_reward") as u32;
    let gold_reward = entity.get_int("gold_reward") as u32;

    loop {
        crate::ui::print_separator();
        println!(
            "⚔  {} (HP:{}/{})  vs  {} (HP:{}/{})",
            player.name,
            player.hp,
            player.max_hp,
            entity.name,
            enemy_hp,
            enemy_max_hp
        );
        crate::ui::print_separator();

        let choice = crate::ui::print_menu("战斗", &["攻击", "逃跑"]);

        if choice == 1 {
            // Flee attempt: 25% chance
            let mut rng = rand::thread_rng();
            if rng.gen_bool(0.25) {
                crate::ui::print_message("你成功逃脱了！");
                entity.set_int("hp", enemy_hp as i64);
                return CombatResult::Fled;
            }
            crate::ui::print_message("逃跑失败！");
        } else {
            let dmg = calc_damage(player.attack, enemy_def);
            enemy_hp = (enemy_hp - dmg).max(0);
            crate::ui::print_message(&format!(
                "你对 {} 造成了 {} 点伤害。",
                entity.name, dmg
            ));
        }

        if enemy_hp <= 0 {
            crate::ui::print_message(&format!("你击败了 {}！", entity.name));
            entity.set_int("hp", 0);
            return CombatResult::Victory {
                exp: exp_reward,
                gold: gold_reward,
            };
        }

        // Enemy turn
        let dmg = calc_damage(enemy_atk, player.defense);
        player.take_damage(dmg);
        crate::ui::print_message(&format!(
            "{} 对你造成了 {} 点伤害。",
            entity.name, dmg
        ));

        if !player.is_alive() {
            crate::ui::print_message("你被击败了...");
            entity.set_int("hp", enemy_hp as i64);
            return CombatResult::Defeat;
        }
    }
}

/// Create monster entities for a given area level.
///
/// Returns a vec of `(name, state_pairs)` that can be fed to `World::spawn_entity`.
pub fn generate_monsters(
    area_level: u32,
) -> Vec<(String, Vec<(&'static str, i64)>)> {
    let scale = area_level as i64;
    vec![
        (
            format!("哥布林(Lv{})", area_level),
            vec![
                ("hp", 20 + scale * 10),
                ("max_hp", 20 + scale * 10),
                ("attack", 5 + scale * 2),
                ("defense", 2 + scale),
                ("exp_reward", 15 + (area_level as i64) * 10),
                ("gold_reward", 5 + (area_level as i64) * 3),
            ],
        ),
        (
            format!("狼(Lv{})", area_level),
            vec![
                ("hp", 15 + scale * 12),
                ("max_hp", 15 + scale * 12),
                ("attack", 7 + scale * 2),
                ("defense", 1 + scale),
                ("exp_reward", 20 + (area_level as i64) * 10),
                ("gold_reward", 3 + (area_level as i64) * 2),
            ],
        ),
        (
            format!("强盗(Lv{})", area_level),
            vec![
                ("hp", 25 + scale * 8),
                ("max_hp", 25 + scale * 8),
                ("attack", 6 + scale * 3),
                ("defense", 3 + scale * 2),
                ("exp_reward", 25 + (area_level as i64) * 12),
                ("gold_reward", 10 + (area_level as i64) * 5),
            ],
        ),
        (
            format!("石像鬼(Lv{})", area_level),
            vec![
                ("hp", 40 + scale * 15),
                ("max_hp", 40 + scale * 15),
                ("attack", 4 + scale * 2),
                ("defense", 6 + scale * 2),
                ("exp_reward", 30 + (area_level as i64) * 15),
                ("gold_reward", 8 + (area_level as i64) * 4),
            ],
        ),
        (
            format!("黑暗法师(Lv{})", area_level),
            vec![
                ("hp", 18 + scale * 8),
                ("max_hp", 18 + scale * 8),
                ("attack", 10 + scale * 4),
                ("defense", 1 + scale),
                ("exp_reward", 35 + (area_level as i64) * 18),
                ("gold_reward", 12 + (area_level as i64) * 6),
            ],
        ),
    ]
}
