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
        println!("Enter your character name:");
        let name = crate::ui::read_line();
        let name = if name.trim().is_empty() {
            "Hero".to_string()
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
                "Main Menu",
                &[
                    "Explore",
                    "Farm",
                    "Breed Animals",
                    "Rest (Heal 30 HP for 20g)",
                    "View Status",
                    "Save Game",
                    "Load Game",
                    "Quit",
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
                        Ok(()) => crate::ui::print_message("Game saved!"),
                        Err(e) => crate::ui::print_message(&format!("Save error: {}", e)),
                    }
                    crate::ui::wait_for_enter();
                }
                6 => {
                    match self.load_game() {
                        Ok(()) => crate::ui::print_message("Game loaded!"),
                        Err(e) => crate::ui::print_message(&format!("Load error: {}", e)),
                    }
                    crate::ui::wait_for_enter();
                }
                7 => {
                    crate::ui::print_message("Goodbye!");
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

        println!("Available areas:");
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
        opts.push("Back");

        let choice = crate::ui::print_menu("Choose Area", &opts);

        if choice >= self.areas.len() {
            return;
        }

        let area_name = self.areas[choice].name.clone();
        crate::ui::print_message(&format!("Exploring {}...", area_name));

        match explore(&self.player, choice) {
            Ok(ExploreResult::Enemy(mut enemy)) => {
                crate::ui::print_message(&format!("A {} appears!", enemy.name));
                crate::ui::wait_for_enter();

                let result = run_combat(&mut self.player, &mut enemy);
                match result {
                    CombatResult::Victory { exp, gold } => {
                        self.player.gold += gold;
                        let leveled = self.player.gain_exp(exp);
                        crate::ui::print_message(&format!(
                            "Victory! Gained {}exp and {}g.",
                            exp, gold
                        ));
                        if leveled {
                            crate::ui::print_message(&format!(
                                "Level up! You are now level {}!",
                                self.player.level
                            ));
                        }
                    }
                    CombatResult::Defeat => {
                        crate::ui::print_message("You were defeated. Healing to 1 HP...");
                        self.player.hp = 1;
                    }
                    CombatResult::Fled => {
                        crate::ui::print_message("You escaped safely.");
                    }
                }
            }
            Ok(ExploreResult::Gold(gold)) => {
                self.player.gold += gold;
                crate::ui::print_message(&format!("Found {}g on the ground!", gold));
            }
            Ok(ExploreResult::Item(item)) => {
                crate::ui::print_message(&format!("Found a {}! (Sold for 20g)", item));
                self.player.gold += 20;
            }
            Ok(ExploreResult::Nothing) => {
                crate::ui::print_message("Nothing of interest found.");
            }
            Err(e) => {
                crate::ui::print_message(&format!("Cannot explore: {}", e));
            }
        }

        crate::ui::wait_for_enter();
    }

    fn farm_menu(&mut self) {
        loop {
            crate::ui::clear_screen();
            crate::ui::print_header();

            println!("=== Farm Plots ===");
            for (i, plot) in self.farm.plots.iter().enumerate() {
                match plot {
                    Some(crop) => {
                        let task_done = crop
                            .task_id
                            .map(|id| self.scheduler.is_task_completed(id))
                            .unwrap_or(false);
                        let status = if task_done { "Ready!" } else { "Growing..." };
                        println!("  Plot {}: {} - {}", i + 1, crop.name, status);
                    }
                    None => println!("  Plot {}: Empty", i + 1),
                }
            }
            crate::ui::print_separator();

            let choice = crate::ui::print_menu("Farm Menu", &["Plant Crop", "Harvest Crop", "Back"]);

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
        opts.push("Back");
        let crop_choice = crate::ui::print_menu("Select Crop", &opts);
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
            crate::ui::print_message("No empty plots available!");
            crate::ui::wait_for_enter();
            return;
        }

        let plot_labels: Vec<String> = empty_plots.iter().map(|i| format!("Plot {}", i + 1)).collect();
        let plot_opts: Vec<&str> = plot_labels.iter().map(|s| s.as_str()).collect();
        let plot_choice = crate::ui::print_menu("Select Plot", &plot_opts);
        let plot_idx = empty_plots[plot_choice];

        match self.farm.plant(plot_idx, crop_choice, &self.scheduler) {
            Ok(_) => crate::ui::print_message("Crop planted!"),
            Err(e) => crate::ui::print_message(&format!("Error: {}", e)),
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
            crate::ui::print_message("No crops are ready to harvest yet.");
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
        let choice = crate::ui::print_menu("Harvest which crop?", &plot_opts);
        let plot_idx = ready_plots[choice];

        match self.farm.harvest(plot_idx, &self.scheduler) {
            Some(gold) => {
                self.player.gold += gold;
                crate::ui::print_message(&format!("Harvested! Earned {}g.", gold));
            }
            None => crate::ui::print_message("Crop not ready yet."),
        }
        crate::ui::wait_for_enter();
    }

    fn breed_menu(&mut self) {
        loop {
            crate::ui::clear_screen();
            crate::ui::print_header();

            println!("=== Animals ===");
            for (i, animal) in self.farm.animals.iter().enumerate() {
                if animal.breeding {
                    let task_done = animal
                        .task_id
                        .map(|id| self.scheduler.is_task_completed(id))
                        .unwrap_or(false);
                    let status = if task_done { "Ready to collect!" } else { "Breeding..." };
                    println!("  {}. {} - {}", i + 1, animal.name, status);
                } else {
                    println!("  {}. {} - Idle ({}s, {}g)", i + 1, animal.name, animal.breed_time_secs, animal.yield_gold);
                }
            }
            crate::ui::print_separator();

            let choice = crate::ui::print_menu("Breed Menu", &["Start Breeding", "Collect Animal", "Back"]);

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
            crate::ui::print_message("All animals are already breeding.");
            crate::ui::wait_for_enter();
            return;
        }

        let labels: Vec<String> = idle
            .iter()
            .map(|i| self.farm.animals[*i].name.clone())
            .collect();
        let opts: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
        let choice = crate::ui::print_menu("Select Animal", &opts);
        let animal_idx = idle[choice];

        match self.farm.start_breeding(animal_idx, &self.scheduler) {
            Ok(_) => crate::ui::print_message("Breeding started!"),
            Err(e) => crate::ui::print_message(&format!("Error: {}", e)),
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
            crate::ui::print_message("No animals ready to collect yet.");
            crate::ui::wait_for_enter();
            return;
        }

        let labels: Vec<String> = ready
            .iter()
            .map(|i| self.farm.animals[*i].name.clone())
            .collect();
        let opts: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
        let choice = crate::ui::print_menu("Collect which animal?", &opts);
        let animal_idx = ready[choice];

        match self.farm.collect_animal(animal_idx, &self.scheduler) {
            Some(gold) => {
                self.player.gold += gold;
                crate::ui::print_message(&format!("Collected! Earned {}g.", gold));
            }
            None => crate::ui::print_message("Animal not ready yet."),
        }
        crate::ui::wait_for_enter();
    }

    fn rest_menu(&mut self) {
        crate::ui::clear_screen();
        crate::ui::print_header();
        crate::ui::print_player_status(&self.player);
        if self.player.hp >= self.player.max_hp {
            crate::ui::print_message("You are already at full HP!");
        } else if self.player.gold < 20 {
            crate::ui::print_message("Not enough gold to rest. (Need 20g)");
        } else {
            self.player.gold -= 20;
            self.player.heal(30);
            crate::ui::print_message(&format!(
                "You rest and recover. HP: {}/{}",
                self.player.hp, self.player.max_hp
            ));
        }
        crate::ui::wait_for_enter();
    }

    fn status_menu(&self) {
        crate::ui::clear_screen();
        crate::ui::print_header();
        crate::ui::print_player_status(&self.player);
        println!("Farm Plots: {}", self.farm.plots.len());
        let occupied = self.farm.plots.iter().filter(|p| p.is_some()).count();
        println!("  Occupied: {}  |  Empty: {}", occupied, self.farm.plots.len() - occupied);
        println!("Animals: {}", self.farm.animals.len());
        for animal in &self.farm.animals {
            let status = if animal.breeding { "Breeding" } else { "Idle" };
            println!("  {} - {}", animal.name, status);
        }
        crate::ui::wait_for_enter();
    }
}
