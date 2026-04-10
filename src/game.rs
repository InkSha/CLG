//! Game loop — orchestrates the three layers.
//!
//! The game loop reads commands from stdin, dispatches them through the
//! command system (which operates on the VFS interface layer), and the VFS
//! translates to engine operations.  Persistence is explicitly triggered
//! by the `save` command.

use crate::command::{self, ExecResult};
use crate::engine::area::DEFAULT_START_AREA;
use crate::engine::player::Player;
use crate::engine::World;
use crate::persistence;
use crate::vfs::Vfs;

/// Top-level game state.
pub struct Game {
    world: World,
    vfs: Vfs,
}

impl Game {
    /// Initialize a new or restored game.
    pub fn new() -> Self {
        crate::ui::clear_screen();
        crate::ui::print_header();

        // Layer 3: load configuration.
        persistence::init_config();
        let areas = persistence::load_areas();
        let crop_types = persistence::load_crop_types();
        let animal_types = persistence::load_animal_types();

        // Try to load a saved game.
        if let Some(world) = persistence::load_game(&areas, &crop_types) {
            println!("欢迎回来，{}！", world.player.name);
            let start = world.player_area.clone();
            let vfs = Vfs::new(&start);
            return Game { world, vfs };
        }

        // New game: prompt for character name.
        println!("请输入你的角色名：");
        let name = crate::ui::read_line();
        let name = if name.trim().is_empty() {
            "勇者".to_string()
        } else {
            name.trim().to_string()
        };
        let player = Player::new(name);

        let world = World::new(
            player,
            DEFAULT_START_AREA,
            areas,
            crop_types,
            animal_types,
        );
        let vfs = Vfs::new(DEFAULT_START_AREA);

        Game { world, vfs }
    }

    /// Main game loop.
    pub fn run(&mut self) {
        println!(
            "\n欢迎来到 CLG — 命令行教学游戏！\n输入 help 查看可用命令。\n"
        );

        loop {
            // Print prompt.
            let prompt = format!(
                "{}@{} {} $ ",
                self.world.player.name,
                self.world.player_area,
                self.vfs.pwd()
            );
            print!("{}", prompt);
            let _ = std::io::Write::flush(&mut std::io::stdout());

            let input = crate::ui::read_line();
            let input = input.trim();
            if input.is_empty() {
                continue;
            }

            // Parse command.
            let cmd = match command::parse(input) {
                Ok(cmd) => cmd,
                Err(e) => {
                    if !e.is_empty() {
                        println!("{}", e);
                    }
                    continue;
                }
            };

            // Handle save specially (needs access to world).
            if matches!(cmd, command::Command::Save) {
                match persistence::save_game(&self.world) {
                    Ok(()) => println!("✅ 游戏已保存！"),
                    Err(e) => println!("保存错误：{}", e),
                }
                continue;
            }

            // Execute command through the interface layer.
            match command::execute(cmd, &mut self.world, &mut self.vfs) {
                ExecResult::Output(text) => {
                    if !text.is_empty() {
                        println!("{}", text);
                    }
                }
                ExecResult::Done => {}
                ExecResult::Quit => {
                    println!("再见！");
                    // Auto-save on quit.
                    let _ = persistence::save_game(&self.world);
                    return;
                }
            }
        }
    }
}
