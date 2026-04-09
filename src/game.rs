use crate::{
    combat::{run_combat, CombatResult},
    exploration::{explore, get_areas, Area, ExploreResult},
    farming::Farm,
    player::Player,
    scheduler::Scheduler,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SaveData {
    player: Player,
    farm: Farm,
}

pub struct GameState {
    player: Player,
    farm: Farm,
    scheduler: Scheduler,
    areas: Vec<Area>,
}

impl GameState {
    pub fn new() -> Self {
        crate::ui::clear_screen();
        crate::ui::print_header();
        println!("请输入你的角色名：");
        let name = crate::ui::read_line();
        let name = if name.trim().is_empty() {
            "勇者".to_string()
        } else {
            name.trim().to_string()
        };

        GameState {
            player: Player::new(name),
            farm: Farm::new(),
            scheduler: Scheduler::new(),
            areas: get_areas(),
        }
    }

    pub fn run(&mut self) {
        loop {
            crate::ui::clear_screen();
            crate::ui::print_header();
            crate::ui::print_player_status(&self.player);

            let choice = crate::ui::print_menu(
                "主菜单",
                &[
                    "探索",
                    "农场",
                    "繁殖动物",
                    "休息（花费 20 金币回复 30 生命值）",
                    "查看状态",
                    "保存游戏",
                    "读取游戏",
                    "退出",
                ],
            );

            match choice {
                0 => self.explore_menu(),
                1 => self.farm_menu(),
                2 => self.breed_menu(),
                3 => self.rest_menu(),
                4 => self.status_menu(),
                5 => {
                    match self.save_game() {
                        Ok(()) => crate::ui::print_message("游戏已保存！"),
                        Err(e) => crate::ui::print_message(&format!("保存错误：{}", e)),
                    }
                    crate::ui::wait_for_enter();
                }
                6 => {
                    match self.load_game() {
                        Ok(()) => crate::ui::print_message("游戏已读取！"),
                        Err(e) => crate::ui::print_message(&format!("读取错误：{}", e)),
                    }
                    crate::ui::wait_for_enter();
                }
                7 => {
                    crate::ui::print_message("再见！");
                    return;
                }
                _ => {}
            }
        }
    }

    fn save_game(&self) -> Result<(), String> {
        let data = SaveData {
            player: self.player.clone(),
            farm: Farm {
                plots: self.farm.plots.clone(),
                animals: self.farm.animals.clone(),
            },
        };
        let json = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
        std::fs::write("save.json", json).map_err(|e| e.to_string())
    }

    fn load_game(&mut self) -> Result<(), String> {
        let json = std::fs::read_to_string("save.json").map_err(|e| e.to_string())?;
        let data: SaveData = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        self.player = data.player;
        self.farm = data.farm;
        Ok(())
    }

    fn explore_menu(&mut self) {
        crate::ui::clear_screen();
        crate::ui::print_header();

        println!("可用区域：");
        for (i, area) in self.areas.iter().enumerate() {
            println!(
                "  {}. {} (Lv.{} req) - {}",
                i + 1,
                area.name,
                area.level_req,
                area.description
            );
        }

        let area_names: Vec<String> = self.areas.iter().map(|a| a.name.clone()).collect();
        let mut opts: Vec<&str> = area_names.iter().map(|s| s.as_str()).collect();
        opts.push("返回");

        let choice = crate::ui::print_menu("选择区域", &opts);

        if choice >= self.areas.len() {
            return;
        }

        let area_name = self.areas[choice].name.clone();
        crate::ui::print_message(&format!("正在探索 {}...", area_name));

        match explore(&self.player, choice) {
            Ok(ExploreResult::Enemy(mut enemy)) => {
                crate::ui::print_message(&format!("遭遇了 {}！", enemy.name));
                crate::ui::wait_for_enter();

                let result = run_combat(&mut self.player, &mut enemy);
                match result {
                    CombatResult::Victory { exp, gold } => {
                        self.player.gold += gold;
                        let leveled = self.player.gain_exp(exp);
                        crate::ui::print_message(&format!(
                            "胜利！获得 {} 经验和 {} 金币。",
                            exp, gold
                        ));
                        if leveled {
                            crate::ui::print_message(&format!(
                                "升级了！当前等级 {}！",
                                self.player.level
                            ));
                        }
                    }
                    CombatResult::Defeat => {
                        crate::ui::print_message("你被击败了，生命值恢复至 1 点...");
                        self.player.hp = 1;
                    }
                    CombatResult::Fled => {
                        crate::ui::print_message("你安全逃脱了。");
                    }
                }
            }
            Ok(ExploreResult::Gold(gold)) => {
                self.player.gold += gold;
                crate::ui::print_message(&format!("在地上发现了 {} 金币！", gold));
            }
            Ok(ExploreResult::Item(item)) => {
                crate::ui::print_message(&format!("发现了 {}！（卖出 20 金币）", item));
                self.player.gold += 20;
            }
            Ok(ExploreResult::Nothing) => {
                crate::ui::print_message("什么都没有发现。");
            }
            Err(e) => {
                crate::ui::print_message(&format!("无法探索：{}", e));
            }
        }

        crate::ui::wait_for_enter();
    }

    fn farm_menu(&mut self) {
        loop {
            crate::ui::clear_screen();
            crate::ui::print_header();

            println!("=== 农场地块 ===");
            for (i, plot) in self.farm.plots.iter().enumerate() {
                match plot {
                    Some(crop) => {
                        let task_done = crop
                            .task_id
                            .map(|id| self.scheduler.is_task_completed(id))
                            .unwrap_or(false);
                        let status = if task_done { "可以收获！" } else { "生长中..." };
                        println!("  地块 {}：{} - {}", i + 1, crop.name, status);
                    }
                    None => println!("  地块 {}：空地", i + 1),
                }
            }
            crate::ui::print_separator();

            let choice = crate::ui::print_menu("农场菜单", &["种植作物", "收获作物", "返回"]);

            match choice {
                0 => self.plant_crop(),
                1 => self.harvest_crop(),
                2 => break,
                _ => {}
            }
        }
    }

    fn plant_crop(&mut self) {
        let crop_types = Farm::get_crop_types();
        let crop_names: Vec<String> = crop_types
            .iter()
            .map(|(n, secs, gold)| format!("{} ({}s -> {}g)", n, secs, gold))
            .collect();
        let mut opts: Vec<&str> = crop_names.iter().map(|s| s.as_str()).collect();
        opts.push("返回");
        let crop_choice = crate::ui::print_menu("选择作物", &opts);
        if crop_choice >= crop_types.len() {
            return;
        }

        // Select empty plot
        let empty_plots: Vec<usize> = self
            .farm
            .plots
            .iter()
            .enumerate()
            .filter(|(_, p)| p.is_none())
            .map(|(i, _)| i)
            .collect();

        if empty_plots.is_empty() {
            crate::ui::print_message("没有空闲地块！");
            crate::ui::wait_for_enter();
            return;
        }

        let plot_labels: Vec<String> = empty_plots.iter().map(|i| format!("地块 {}", i + 1)).collect();
        let plot_opts: Vec<&str> = plot_labels.iter().map(|s| s.as_str()).collect();
        let plot_choice = crate::ui::print_menu("选择地块", &plot_opts);
        let plot_idx = empty_plots[plot_choice];

        match self.farm.plant(plot_idx, crop_choice, &self.scheduler) {
            Ok(_) => crate::ui::print_message("作物已种植！"),
            Err(e) => crate::ui::print_message(&format!("错误：{}", e)),
        }
        crate::ui::wait_for_enter();
    }

    fn harvest_crop(&mut self) {
        let ready_plots: Vec<usize> = self
            .farm
            .plots
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                p.as_ref()
                    .and_then(|c| c.task_id)
                    .map(|id| self.scheduler.is_task_completed(id))
                    .unwrap_or(false)
            })
            .map(|(i, _)| i)
            .collect();

        if ready_plots.is_empty() {
            crate::ui::print_message("还没有作物可以收获。");
            crate::ui::wait_for_enter();
            return;
        }

        let plot_labels: Vec<String> = ready_plots
            .iter()
            .map(|i| {
                let crop = self.farm.plots[*i].as_ref().unwrap();
                format!("Plot {} - {}", i + 1, crop.name)
            })
            .collect();
        let plot_opts: Vec<&str> = plot_labels.iter().map(|s| s.as_str()).collect();
        let choice = crate::ui::print_menu("收获哪个作物？", &plot_opts);
        let plot_idx = ready_plots[choice];

        match self.farm.harvest(plot_idx, &self.scheduler) {
            Some(gold) => {
                self.player.gold += gold;
                crate::ui::print_message(&format!("收获成功！获得 {} 金币。", gold));
            }
            None => crate::ui::print_message("作物尚未成熟。"),
        }
        crate::ui::wait_for_enter();
    }

    fn breed_menu(&mut self) {
        loop {
            crate::ui::clear_screen();
            crate::ui::print_header();

            println!("=== 动物 ===");
            for (i, animal) in self.farm.animals.iter().enumerate() {
                if animal.breeding {
                    let task_done = animal
                        .task_id
                        .map(|id| self.scheduler.is_task_completed(id))
                        .unwrap_or(false);
                    let status = if task_done { "可以收集！" } else { "繁殖中..." };
                    println!("  {}. {} - {}", i + 1, animal.name, status);
                } else {
                    println!("  {}. {} - 空闲（{}秒，{}金币）", i + 1, animal.name, animal.breed_time_secs, animal.yield_gold);
                }
            }
            crate::ui::print_separator();

            let choice = crate::ui::print_menu("繁殖菜单", &["开始繁殖", "收集动物", "返回"]);

            match choice {
                0 => self.start_breeding(),
                1 => self.collect_animal(),
                2 => break,
                _ => {}
            }
        }
    }

    fn start_breeding(&mut self) {
        let idle: Vec<usize> = self
            .farm
            .animals
            .iter()
            .enumerate()
            .filter(|(_, a)| !a.breeding)
            .map(|(i, _)| i)
            .collect();

        if idle.is_empty() {
            crate::ui::print_message("所有动物都在繁殖中。");
            crate::ui::wait_for_enter();
            return;
        }

        let labels: Vec<String> = idle
            .iter()
            .map(|i| self.farm.animals[*i].name.clone())
            .collect();
        let opts: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
        let choice = crate::ui::print_menu("选择动物", &opts);
        let animal_idx = idle[choice];

        match self.farm.start_breeding(animal_idx, &self.scheduler) {
            Ok(_) => crate::ui::print_message("繁殖已开始！"),
            Err(e) => crate::ui::print_message(&format!("错误：{}", e)),
        }
        crate::ui::wait_for_enter();
    }

    fn collect_animal(&mut self) {
        let ready: Vec<usize> = self
            .farm
            .animals
            .iter()
            .enumerate()
            .filter(|(_, a)| {
                a.breeding
                    && a.task_id
                        .map(|id| self.scheduler.is_task_completed(id))
                        .unwrap_or(false)
            })
            .map(|(i, _)| i)
            .collect();

        if ready.is_empty() {
            crate::ui::print_message("还没有动物可以收集。");
            crate::ui::wait_for_enter();
            return;
        }

        let labels: Vec<String> = ready
            .iter()
            .map(|i| self.farm.animals[*i].name.clone())
            .collect();
        let opts: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
        let choice = crate::ui::print_menu("收集哪只动物？", &opts);
        let animal_idx = ready[choice];

        match self.farm.collect_animal(animal_idx, &self.scheduler) {
            Some(gold) => {
                self.player.gold += gold;
                crate::ui::print_message(&format!("收集成功！获得 {} 金币。", gold));
            }
            None => crate::ui::print_message("动物还未准备好。"),
        }
        crate::ui::wait_for_enter();
    }

    fn rest_menu(&mut self) {
        crate::ui::clear_screen();
        crate::ui::print_header();
        crate::ui::print_player_status(&self.player);
        if self.player.hp >= self.player.max_hp {
            crate::ui::print_message("你的生命值已满！");
        } else if self.player.gold < 20 {
            crate::ui::print_message("金币不足，无法休息。（需要 20 金币）");
        } else {
            self.player.gold -= 20;
            self.player.heal(30);
            crate::ui::print_message(&format!(
                "你休息并恢复了体力。生命值：{}/{}",
                self.player.hp, self.player.max_hp
            ));
        }
        crate::ui::wait_for_enter();
    }

    fn status_menu(&self) {
        crate::ui::clear_screen();
        crate::ui::print_header();
        crate::ui::print_player_status(&self.player);
        println!("农场地块：{}", self.farm.plots.len());
        let occupied = self.farm.plots.iter().filter(|p| p.is_some()).count();
        println!("  已占用：{}  |  空闲：{}", occupied, self.farm.plots.len() - occupied);
        println!("动物：{}", self.farm.animals.len());
        for animal in &self.farm.animals {
            let status = if animal.breeding { "繁殖中" } else { "空闲" };
            println!("  {} - {}", animal.name, status);
        }
        crate::ui::wait_for_enter();
    }
}
