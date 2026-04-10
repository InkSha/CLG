use crate::{
    actions::{ActionMap, BuiltinCmd},
    combat::{run_combat, CombatResult},
    exploration::{explore, Area, ExploreResult},
    farming::{CropType, Farm},
    player::Player,
    ui_template::{render_template, scope_matches, UiContext, UiTemplate},
    world::{WorldEvent, WorldManager, FARM_AREA, PLAYER_FILE, DEFAULT_START_AREA},
};

pub struct GameState {
    player: Player,
    farm: Farm,
    world: WorldManager,
    /// The area directory the player is currently in.
    current_area: String,
    areas: Vec<Area>,
    /// Crop types loaded from config.
    crop_types: Vec<CropType>,
    /// Active UI template (loaded from world/config/ui.yaml).
    ui_template: UiTemplate,
}

impl GameState {
    pub fn new() -> Self {
        crate::ui::clear_screen();
        crate::ui::print_header();

        // Initialise the filesystem-driven world.
        let world = WorldManager::new().expect("无法初始化世界文件系统");

        // Write default config files (areas, crops, animals, action.yaml) if absent.
        world.init_config().expect("无法初始化配置文件");

        // Load areas from config (falls back to built-in defaults).
        let areas = world.load_areas_config();
        let area_names: Vec<&str> = areas.iter().map(|a| a.name.as_str()).collect();

        // Create area directories + area.yaml metadata files.
        world.init_areas(&areas).expect("无法创建区域目录");

        // Write built-in template files (once, if absent) and apply any templates.
        world.init_templates().expect("无法初始化模板目录");
        let generated = world.scan_and_apply_templates();
        if !generated.is_empty() {
            println!("📋 从模板生成了以下实体文件：");
            for (tmpl, out) in &generated {
                println!("   {} → {}", tmpl, out);
            }
        }

        // Find the area that already holds player.yaml, or default to 森林.
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

        // Load crop types from config.
        let crop_types = world.load_crop_types();

        // Load the UI template.
        let ui_template = world.load_ui_template();

        // Init farm directory; load state from files.
        let farm_template = world.make_default_farm();
        world.init_farm(&farm_template).expect("无法初始化农场目录");
        let farm = world.load_farm(&farm_template).unwrap_or(farm_template);

        println!(
            "\n📁 世界目录已就绪：world/\n   玩家文件：world/{}/player.yaml",
            current_area
        );
        crate::ui::wait_for_enter();

        GameState {
            player,
            farm,
            world,
            current_area,
            areas,
            crop_types,
            ui_template,
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

    /// Render the UI template for the current area and print it to stdout.
    ///
    /// Only renders when the template's scope matches the current area path.
    fn print_ui(&self) {
        let area_path = format!("world/{}/", self.current_area);
        if !scope_matches(&self.ui_template, &area_path) {
            // Template scope doesn't apply here; fall back to the classic display.
            crate::ui::print_player_status(&self.player);
            return;
        }

        // Build pre-rendered includes.
        let include_templates = self.world.load_ui_includes(&self.ui_template);
        let includes = include_templates
            .iter()
            .map(|(name, tmpl)| {
                let inner_ctx = UiContext {
                    player: &self.player,
                    current_area: &self.current_area,
                    includes: std::collections::HashMap::new(),
                };
                (name.clone(), render_template(tmpl, &inner_ctx))
            })
            .collect();

        let ctx = UiContext {
            player: &self.player,
            current_area: &self.current_area,
            includes,
        };
        print!("{}", render_template(&self.ui_template, &ctx));
    }

    /// Print any pending filesystem events from the background watcher.
    fn display_world_events(&mut self) {
        let events = self.world.poll_events();
        if !events.is_empty() {
            crate::ui::print_separator();
            println!("📡 [文件系统监听事件]");
            for event in events {
                match event {
                    WorldEvent::PlayerMoved { to_area } => {
                        println!("  📂 player.yaml → world/{}/", to_area);
                    }
                    WorldEvent::EntityCreated { area, filename } => {
                        println!("  📄 创建：world/{}/{}", area, filename);
                    }
                    WorldEvent::EntityRemoved { area, filename } => {
                        println!("  🗑  删除：world/{}/{}", area, filename);
                    }
                    WorldEvent::TemplateChanged { path } => {
                        println!("  📋 模板变更：world/{}", path);
                        // Reload UI template when config/ui.yaml changes.
                        let changed_path = std::path::Path::new(&path);
                        let ui_path = std::path::Path::new("config").join("ui.yaml");
                        if changed_path == ui_path {
                            self.ui_template = self.world.load_ui_template();
                            println!("     🎨 UI 模板已重新加载");
                        } else {
                            match self.world.reapply_template(&path) {
                                Ok(out) => println!("     ✅ 已重新生成：{}", out),
                                Err(e) => println!("     ⚠️  重新生成失败：{}", e),
                            }
                        }
                    }
                }
            }
            crate::ui::print_separator();
        }
    }

    // ── Main game loop ────────────────────────────────────────────────────────

    pub fn run(&mut self) {
        loop {
            crate::ui::clear_screen();
            crate::ui::print_header();
            self.print_ui();
            println!("📍 当前位置：{}  (world/{}/)", self.current_area, self.current_area);
            crate::ui::print_separator();

            // Show files in current area.
            let files = self.world.list_area_files(&self.current_area);
            if !files.is_empty() {
                println!("📂 world/{}/", self.current_area);
                for f in &files {
                    println!("   {}", f);
                }
                crate::ui::print_separator();
            }

            self.display_world_events();

            // Load action map for the current area.
            let action_map = self.world.load_action_map(&self.current_area);
            self.print_actions(&action_map);

            // Read player command.
            print!("\n> ");
            let _ = std::io::Write::flush(&mut std::io::stdout());
            let input = crate::ui::read_line();
            let input = input.trim();

            if input.is_empty() {
                continue;
            }

            // Match input to an action.
            match action_map.match_input(input) {
                Some(cmd) => {
                    if self.execute_cmd(cmd) {
                        return; // Quit
                    }
                }
                None => {
                    println!("❓ 未知指令：\"{}\"。输入 status 查看帮助。", input);
                    crate::ui::wait_for_enter();
                }
            }
        }
    }

    /// Print available actions from the action map.
    fn print_actions(&self, map: &ActionMap) {
        println!("📋 可用指令：");
        for (name, builtin) in map.display_list() {
            println!("  {:20}  {}", name, builtin);
        }
    }

    /// Execute a built-in command.  Returns `true` if the game should quit.
    fn execute_cmd(&mut self, cmd: BuiltinCmd) -> bool {
        match cmd {
            BuiltinCmd::Ls { path } => self.cmd_ls(path),
            BuiltinCmd::Cd { path } => self.cmd_cd(&path),
            BuiltinCmd::Cat { file } => self.cmd_cat(&file),
            BuiltinCmd::EchoTo { content, file } => self.cmd_echo_to(&content, &file),
            BuiltinCmd::Grep { pattern } => self.cmd_grep(&pattern),
            BuiltinCmd::Farm => self.farm_menu(),
            BuiltinCmd::Breed => self.breed_menu(),
            BuiltinCmd::Rest => self.rest_menu(),
            BuiltinCmd::Status => self.status_menu(),
            BuiltinCmd::Save => {
                match self.save_game() {
                    Ok(()) => crate::ui::print_message("✅ 游戏已保存！"),
                    Err(e) => crate::ui::print_message(&format!("保存错误：{}", e)),
                }
                crate::ui::wait_for_enter();
            }
            BuiltinCmd::Quit => {
                crate::ui::print_message("再见！");
                return true;
            }
        }
        false
    }

    // ── Built-in command handlers ─────────────────────────────────────────────

    fn cmd_ls(&self, path: Option<String>) {
        let area = path.as_deref().unwrap_or(&self.current_area);
        let files = self.world.list_area_files(area);
        println!("📂 world/{}/", area);
        if files.is_empty() {
            println!("   （空）");
        } else {
            for f in &files {
                println!("   {}", f);
            }
        }
        crate::ui::wait_for_enter();
    }

    /// `cd <path>` — navigate to another area, triggering an encounter if
    /// moving to a named area different from the current one.
    fn cmd_cd(&mut self, path: &str) {
        let target = match path {
            "~" => DEFAULT_START_AREA.to_string(),
            ".." => DEFAULT_START_AREA.to_string(),
            other => other.to_string(),
        };

        // Validate the target is a known area.
        let area = self.areas.iter().find(|a| a.name == target).cloned();
        let Some(area) = area else {
            println!("❌ 未知区域：\"{}\"", target);
            println!("   可用区域：");
            for a in &self.areas {
                println!("     {}  (Lv.{} 要求)", a.name, a.level_req);
            }
            crate::ui::wait_for_enter();
            return;
        };

        let is_new_area = target != self.current_area;
        let trigger_explore = is_new_area && path != "~" && path != "..";

        if is_new_area {
            crate::ui::print_message(&format!(
                "📂 正在将 player.yaml 从 world/{} 移动到 world/{}...",
                self.current_area, target
            ));
            match self.world.move_player(&self.current_area, &target) {
                Ok(()) => {
                    self.current_area = target.clone();
                }
                Err(e) => {
                    crate::ui::print_message(&format!("移动失败：{}", e));
                    crate::ui::wait_for_enter();
                    return;
                }
            }
        } else {
            println!("📍 已在 {} 中。", self.current_area);
        }

        if trigger_explore {
            self.run_explore(&area);
        } else {
            crate::ui::wait_for_enter();
        }
    }

    fn cmd_cat(&self, file: &str) {
        if file.is_empty() {
            println!("用法：cat <文件名>");
            crate::ui::wait_for_enter();
            return;
        }
        match self.world.read_entity_raw(&self.current_area, file) {
            Ok(content) => {
                println!("📄 world/{}/{}:", self.current_area, file);
                println!("{}", content);
            }
            Err(e) => println!("❌ 无法读取文件：{}", e),
        }
        crate::ui::wait_for_enter();
    }

    fn cmd_echo_to(&self, content: &str, file: &str) {
        if file.is_empty() {
            println!("用法：echo <内容> > <文件名>");
            crate::ui::wait_for_enter();
            return;
        }
        match self.world.write_entity_raw(&self.current_area, file, content) {
            Ok(()) => println!("✅ 已写入 world/{}/{}", self.current_area, file),
            Err(e) => println!("❌ 写入失败：{}", e),
        }
        crate::ui::wait_for_enter();
    }

    fn cmd_grep(&self, pattern: &str) {
        if pattern.is_empty() {
            println!("用法：find <关键词>");
            crate::ui::wait_for_enter();
            return;
        }
        let results = self.world.search_area(&self.current_area, pattern);
        println!("🔍 在 world/{}/ 中搜索 \"{}\"：", self.current_area, pattern);
        if results.is_empty() {
            println!("   未找到匹配结果。");
        } else {
            for (filename, lines) in &results {
                println!("  📄 {}", filename);
                for (lineno, text) in lines {
                    println!("     {}:  {}", lineno, text);
                }
            }
        }
        crate::ui::wait_for_enter();
    }

    // ── Exploration encounter ─────────────────────────────────────────────────

    /// Run an exploration encounter in `area`.
    fn run_explore(&mut self, area: &Area) {
        crate::ui::print_message(&format!("正在探索 {}...", area.name));

        match explore(&self.player, area) {
            Ok(ExploreResult::Enemy(mut enemy)) => {
                let enemy_file = format!("enemy_{}.yaml", sanitize_filename(&enemy.name));
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
                let item_file = format!("item_{}.yaml", sanitize_filename(&item));
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

    // ── Save / Load ───────────────────────────────────────────────────────────

    fn save_game(&self) -> Result<(), String> {
        self.sync_state();
        Ok(())
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
                            "  地块 {}：{} - {}  [world/{}/plot_{}.yaml]",
                            i + 1,
                            crop.name,
                            status,
                            FARM_AREA,
                            i
                        );
                    }
                    None => println!(
                        "  地块 {}：空地  [world/{}/plot_{}.yaml]",
                        i + 1,
                        FARM_AREA,
                        i
                    ),
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
        let crop_names: Vec<String> = self
            .crop_types
            .iter()
            .map(|c| format!("{} ({}s → {}g)", c.name, c.grow_time_secs, c.yield_gold))
            .collect();
        let mut opts: Vec<&str> = crop_names.iter().map(|s| s.as_str()).collect();
        opts.push("返回");
        let crop_choice = crate::ui::print_menu("选择作物", &opts);
        if crop_choice >= self.crop_types.len() {
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

        let crop_type = self.crop_types[crop_choice].clone();
        match self.farm.plant(plot_idx, &crop_type) {
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
                    println!(
                        "  {}. {} - {}  [world/{}/{}.yaml]",
                        i + 1,
                        animal.name,
                        status,
                        FARM_AREA,
                        animal.name
                    );
                } else {
                    println!(
                        "  {}. {} - 空闲（{}秒，{}金币）  [world/{}/{}.yaml]",
                        i + 1,
                        animal.name,
                        animal.breed_time_secs,
                        animal.yield_gold,
                        FARM_AREA,
                        animal.name
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
        self.print_ui();
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
        self.print_ui();
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
            println!(
                "  {} - {}  [world/{}/{}.yaml]",
                animal.name, status, FARM_AREA, animal.name
            );
        }
        println!("\n可探索区域：");
        for area in &self.areas {
            println!("  {}  (Lv.{} 要求，敌人等级 {})", area.name, area.level_req, area.enemy_level);
        }
        crate::ui::wait_for_enter();
    }
}

/// Sanitize a string for use as a filename component.
fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
