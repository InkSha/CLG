mod game;
mod player;
mod combat;
mod exploration;
mod farming;
mod world;
mod ui;

fn main() {
    let mut game = game::GameState::new();
    game.run();
}
