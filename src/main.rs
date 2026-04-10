mod actions;
mod game;
mod player;
mod combat;
mod exploration;
mod farming;
mod template;
mod world;
mod ui;
mod ui_template;

fn main() {
    let mut game = game::GameState::new();
    game.run();
}
