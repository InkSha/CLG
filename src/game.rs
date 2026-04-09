use crate::{
    combat::{run_combat, CombatResult},
    exploration::{explore, get_areas, Area, ExploreResult},
    farming::Farm,
    player::Player,
    world::{WorldEvent, WorldManager, FARM_AREA, PLAYER_FILE, DEFAULT_START_AREA},
};

pub struct GameState {
    player: Player,
    farm: Farm,
    world: WorldManager,
    /// The area directory the player is currently in.
    current_area: String,
    areas: Vec<Area>,
}

impl GameState {
    pub fn new() -> Self {
        crate::ui::clear_screen();
        crate::ui::print_header();

        // Initialise the filesystem-driven world.
        let world = WorldManager::new().expect("无法初始化世界文件系统");
        let areas = get_areas();
        let area_names: Vec<&str> = areas.iter().map(|a| a.name.as_str()).collect();

        // Create area directories + area.json metadata files.
        world.init_areas(&areas).expect("无法创建区域目录");

        // Find the area that already holds player.json, or default to 森林.
        let current_area = world
            .find_player_area(&area_names)
            .unwrap_or_else(|| DEFAULT_START_AREA.to_string());

        // Load existing player or prompt for a new character.
        let player = if world.entity_exists(&current_area, PLAYER_FILE) {
            match world.read_player(&current_area) {
                Ok(p) => {
                    println!("欢迎回来，{}！当前位置：{}", p.name, current_area);
                    p
                }
                Err(_) => Self::create_new_player(&world, &current_area),
            }
        } else {
            Self::create_new_player(&world, &current_area)
        };

        // Init farm directory; load state from files.
        let farm_template = Farm::new();
        world.init_farm(&farm_template).expect("无法初始化农场目录");
        let farm = world.load_farm(&farm_template).unwrap_or(farm_template);

        println!(
            "\n📁 世界目录已就绪：world/\n   玩家文件：world/{}/player.json",
            current_area
        );
        crate::ui::wait_for_enter();

        GameState {
            player,
            farm,
            world,
            current_area,
            areas,
        }
    }

    /// Prompt for a character name, create a new Player, and write it to disk.
    fn create_new_player(world: &WorldManager, area: &str) -> Player {
        println!("请输入你的角色名：");
        let name = crate::ui::read_line();
        let name = if name.trim().is_empty() {
            "勇者".to_string()
        } else {
            name.trim().to_string()
        };
        let player = Player::new(name);
        world
            .write_player(&player, area)
            .expect("无法写入玩家文件");
        player
    }

    /// Flush player + farm state to the filesystem.
    fn sync_state(&self) {
        if let Err(e) = self.world.write_player(&self.player, &self.current_area) {
            eprintln!("警告：无法同步玩家状态：{}", e);
        }
        if let Err(e) = self.world.sync_farm(&self.farm) {
            eprintln!("警告：无法同步农场状态：{}", e);
        }
    }

    /// Print any pending filesystem events from the background watcher.
    fn display_world_events(&self) {
        let events = self.world.poll_events();
        if !events.is_empty() {
            crate::ui::print_separator();
            println!("📡 [文件系统监听事件]");
            for event in events {
                match event {
                    WorldEvent::PlayerMoved { to_area } => {
                        println!("  📂 player.json → world/{}/", to_area);
                    }
                    WorldEvent::EntityCreated { area, filename } => {
                        println!("  📄 创建：world/{}/{}", area, filename);
                    }
                    WorldEvent::EntityRemoved { area, filename } => {
                        println!("  🗑  删除：world/{}/{}", area, filename);
                    }
                }
            }
            crate::ui::print_separator();
        }
    }

    pub fn run(&mut self) {
        loop {
            crate::ui::clear_screen();
            crate::ui::print_header();
            crate::ui::print_player_status(&self.player);
            println!("当前位置：{}", self.current_area);
            println!(
                "📁 world/{}/player.json",
                self.current_area
            );
            crate::ui::print_separator();

            self.display_world_events();

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

    // ── Save / Load ───────────────────────────────────────────────────────────

    fn save_game(&self) -> Result<(), String> {
        // Persist player + farm to their respective world files.
        self.sync_state();
        Ok(())
    }

    fn load_game(&mut self) -> Result<(), String> {
        // Reload player from the world filesystem.
        self.player = self.world.read_player(&self.current_area)?;
        // Reload farm from its directory files.
        let template = Farm::new();
        self.farm = self.world.load_farm(&template)?;
        Ok(())
    }

    // ── Exploration ───────────────────────────────────────────────────────────

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

        let target_area = self.areas[choice].name.clone();

        // ── Player movement: move player.json to the target area directory ──
        if target_area != self.current_area {
            crate::ui::print_message(&format!(
                "📂 正在将 player.json 从 world/{} 移动到 world/{}...",
                self.current_area, target_area
            ));
            match self.world.move_player(&self.current_area, &target_area) {
                Ok(()) => {
                    self.current_area = target_area.clone();
                }
                Err(e) => {
                    crate::ui::print_message(&format!("移动失败：{}", e));
                    crate::ui::wait_for_enter();
                    return;
                }
            }
        }

        crate::ui::print_message(&format!("正在探索 {}...", target_area));

        match explore(&self.player, choice) {
            Ok(ExploreResult::Enemy(mut enemy)) => {
                // Spawn enemy as a file in the current area directory.
                let enemy_file = format!("enemy_{}.json", sanitize_filename(&enemy.name));
                let enemy_data = serde_json::json!({
                    "type": "enemy",
                    "name": enemy.name,
                    "hp": enemy.hp,
                    "max_hp": enemy.max_hp,
                    "attack": enemy.attack,
                    "defense": enemy.defense,
                    "exp_reward": enemy.exp_reward,
                    "gold_reward": enemy.gold_reward,
                });
                let _ = self
                    .world
                    .write_entity(&self.current_area, &enemy_file, &enemy_data);

                crate::ui::print_message(&format!(
                    "遭遇了 {}！（实体文件：world/{}/{}）",
                    enemy.name, self.current_area, enemy_file
                ));
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
                        // Enemy defeated → remove its file.
                        let _ = self.world.remove_entity(&self.current_area, &enemy_file);
                    }
                    CombatResult::Defeat => {
                        crate::ui::print_message("你被击败了，生命值恢复至 1 点...");
                        self.player.hp = 1;
                        let _ = self.world.remove_entity(&self.current_area, &enemy_file);
                    }
                    CombatResult::Fled => {
                        crate::ui::print_message("你安全逃脱了。");
                        let _ = self.world.remove_entity(&self.current_area, &enemy_file);
                    }
                }
            }
            Ok(ExploreResult::Gold(gold)) => {
                self.player.gold += gold;
                crate::ui::print_message(&format!("在地上发现了 {} 金币！", gold));
            }
            Ok(ExploreResult::Item(item)) => {
                // Create an item file, then immediately "collect" it.
                let item_file = format!("item_{}.json", sanitize_filename(&item));
                let item_data = serde_json::json!({ "type": "item", "name": item });
                let _ = self
                    .world
                    .write_entity(&self.current_area, &item_file, &item_data);
                crate::ui::print_message(&format!("发现了 {}！（卖出 20 金币）", item));
                self.player.gold += 20;
                let _ = self.world.remove_entity(&self.current_area, &item_file);
            }
            Ok(ExploreResult::Nothing) => {
                crate::ui::print_message("什么都没有发现。");
            }
            Err(e) => {
                crate::ui::print_message(&format!("无法探索：{}", e));
            }
        }

        self.sync_state();
        crate::ui::wait_for_enter();
    }

    // ── Farming ───────────────────────────────────────────────────────────────

    fn farm_menu(&mut self) {
        loop {
            crate::ui::clear_screen();
            crate::ui::print_header();

            println!("=== 农场地块 ===");
            for (i, plot) in self.farm.plots.iter().enumerate() {
                match plot {
                    Some(crop) => {
                        let status = if crop.is_ready() { "可以收获！" } else { "生长中..." };
                        println!(
                            "  地块 {}：{} - {}  [world/{}/plot_{}.json]",
                            i + 1,
                            crop.name,
                            status,
                            FARM_AREA,
                            i
                        );
                    }
                    None => println!("  地块 {}：空地  [world/{}/plot_{}.json]", i + 1, FARM_AREA, i),
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

        let plot_labels: Vec<String> =
            empty_plots.iter().map(|i| format!("地块 {}", i + 1)).collect();
        let plot_opts: Vec<&str> = plot_labels.iter().map(|s| s.as_str()).collect();
        let plot_choice = crate::ui::print_menu("选择地块", &plot_opts);
        let plot_idx = empty_plots[plot_choice];

        match self.farm.plant(plot_idx, crop_choice) {
            Ok(()) => {
                crate::ui::print_message("作物已种植！");
                self.world.sync_farm(&self.farm).ok();
            }
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
            .filter(|(_, p)| p.as_ref().map(|c| c.is_ready()).unwrap_or(false))
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

        match self.farm.harvest(plot_idx) {
            Some(gold) => {
                self.player.gold += gold;
                crate::ui::print_message(&format!("收获成功！获得 {} 金币。", gold));
                self.sync_state();
            }
            None => crate::ui::print_message("作物尚未成熟。"),
        }
        crate::ui::wait_for_enter();
    }

    // ── Animal Breeding ───────────────────────────────────────────────────────

    fn breed_menu(&mut self) {
        loop {
            crate::ui::clear_screen();
            crate::ui::print_header();

            println!("=== 动物 ===");
            for (i, animal) in self.farm.animals.iter().enumerate() {
                if animal.breeding {
                    let status = if animal.is_ready() { "可以收集！" } else { "繁殖中..." };
                    println!("  {}. {} - {}  [world/{}/{}.json]", i + 1, animal.name, status, FARM_AREA, animal.name);
                } else {
                    println!(
                        "  {}. {} - 空闲（{}秒，{}金币）  [world/{}/{}.json]",
                        i + 1, animal.name, animal.breed_time_secs, animal.yield_gold, FARM_AREA, animal.name
                    );
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

        match self.farm.start_breeding(animal_idx) {
            Ok(()) => {
                crate::ui::print_message("繁殖已开始！");
                self.world.sync_farm(&self.farm).ok();
            }
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
            .filter(|(_, a)| a.is_ready())
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

        match self.farm.collect_animal(animal_idx) {
            Some(gold) => {
                self.player.gold += gold;
                crate::ui::print_message(&format!("收集成功！获得 {} 金币。", gold));
                self.sync_state();
            }
            None => crate::ui::print_message("动物还未准备好。"),
        }
        crate::ui::wait_for_enter();
    }

    // ── Rest / Status ─────────────────────────────────────────────────────────

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
            self.sync_state();
        }
        crate::ui::wait_for_enter();
    }

    fn status_menu(&self) {
        crate::ui::clear_screen();
        crate::ui::print_header();
        crate::ui::print_player_status(&self.player);
        println!("当前位置：{}  (world/{}/)", self.current_area, self.current_area);
        println!("农场地块：{}", self.farm.plots.len());
        let occupied = self.farm.plots.iter().filter(|p| p.is_some()).count();
        println!(
            "  已占用：{}  |  空闲：{}",
            occupied,
            self.farm.plots.len() - occupied
        );
        println!("动物：{}", self.farm.animals.len());
        for animal in &self.farm.animals {
            let status = if animal.breeding { "繁殖中" } else { "空闲" };
            println!("  {} - {}  [world/{}/{}.json]", animal.name, status, FARM_AREA, animal.name);
        }
        crate::ui::wait_for_enter();
    }
}

/// Sanitize a string for use as a filename component.
fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect()
}
