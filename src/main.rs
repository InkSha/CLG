mod command;
mod engine;
mod game;
mod persistence;
mod ui;
mod vfs;

fn main() {
    let mut game = game::Game::new();
    game.run();
}
