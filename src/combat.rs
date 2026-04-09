use rand::Rng;
use crate::player::Player;

pub struct Enemy {
    pub name: String,
    pub hp: i32,
    pub max_hp: i32,
    pub attack: i32,
    pub defense: i32,
    pub exp_reward: u32,
    pub gold_reward: u32,
}

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

pub fn run_combat(player: &mut Player, enemy: &mut Enemy) -> CombatResult {
    loop {
        // Player turn
        crate::ui::print_separator();
        println!(
            "⚔  {} (HP: {}/{})  vs  {} (HP: {}/{})",
            player.name, player.hp, player.max_hp,
            enemy.name, enemy.hp, enemy.max_hp
        );
        crate::ui::print_separator();

        let choice = crate::ui::print_menu("Combat", &["Attack", "Flee"]);

        if choice == 1 {
            // Flee attempt: 25% chance
            let mut rng = rand::thread_rng();
            if rng.gen_bool(0.25) {
                crate::ui::print_message("You successfully fled!");
                return CombatResult::Fled;
            } else {
                crate::ui::print_message("Failed to flee!");
            }
        } else {
            let dmg = calc_damage(player.attack, enemy.defense);
            enemy.hp = (enemy.hp - dmg).max(0);
            crate::ui::print_message(&format!("You deal {} damage to {}.", dmg, enemy.name));
        }

        if enemy.hp <= 0 {
            crate::ui::print_message(&format!("You defeated {}!", enemy.name));
            return CombatResult::Victory {
                exp: enemy.exp_reward,
                gold: enemy.gold_reward,
            };
        }

        // Enemy turn
        let dmg = calc_damage(enemy.attack, player.defense);
        player.take_damage(dmg);
        crate::ui::print_message(&format!("{} deals {} damage to you.", enemy.name, dmg));

        if !player.is_alive() {
            crate::ui::print_message("You were defeated...");
            return CombatResult::Defeat;
        }
    }
}

pub fn create_enemies_for_area(area_level: u32) -> Vec<Enemy> {
    let scale = area_level as i32;
    vec![
        Enemy {
            name: format!("Goblin (Lv{})", area_level),
            hp: 20 + scale * 10,
            max_hp: 20 + scale * 10,
            attack: 5 + scale * 2,
            defense: 2 + scale,
            exp_reward: 15 + area_level * 10,
            gold_reward: 5 + area_level * 3,
        },
        Enemy {
            name: format!("Wolf (Lv{})", area_level),
            hp: 15 + scale * 12,
            max_hp: 15 + scale * 12,
            attack: 7 + scale * 2,
            defense: 1 + scale,
            exp_reward: 20 + area_level * 10,
            gold_reward: 3 + area_level * 2,
        },
        Enemy {
            name: format!("Bandit (Lv{})", area_level),
            hp: 25 + scale * 8,
            max_hp: 25 + scale * 8,
            attack: 6 + scale * 3,
            defense: 3 + scale * 2,
            exp_reward: 25 + area_level * 12,
            gold_reward: 10 + area_level * 5,
        },
        Enemy {
            name: format!("Stone Golem (Lv{})", area_level),
            hp: 40 + scale * 15,
            max_hp: 40 + scale * 15,
            attack: 4 + scale * 2,
            defense: 6 + scale * 2,
            exp_reward: 30 + area_level * 15,
            gold_reward: 8 + area_level * 4,
        },
        Enemy {
            name: format!("Dark Mage (Lv{})", area_level),
            hp: 18 + scale * 8,
            max_hp: 18 + scale * 8,
            attack: 10 + scale * 4,
            defense: 1 + scale,
            exp_reward: 35 + area_level * 18,
            gold_reward: 12 + area_level * 6,
        },
    ]
}
