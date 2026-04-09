# CLG
Command Line Game

A terminal simulation/management RPG written in Rust. Explore dangerous areas, battle enemies, manage a farm, breed animals, and strengthen your character.

## Features

- **Exploration** — Five tiered areas (Forest → Dark Caves → Haunted Ruins → Volcanic Wastes → Dragon's Peak), each with higher level requirements and tougher enemies.
- **Combat** — Turn-based battles against 5 enemy types scaled per area. Attack or attempt to flee.
- **Farming** — Plant Wheat, Potato, or Carrot in 4 farm plots. Crops grow in real time via a background scheduler thread.
- **Breeding** — Raise Chicken, Cow, and Sheep. Start breeding and collect gold rewards when the timer completes.
- **Character Progression** — Gain EXP and gold from combat/exploration. Level up to increase HP, ATK, and DEF.
- **Rest** — Spend 20g to recover 30 HP at any time.
- **Save / Load** — Game state (player + farm) is persisted to `save.json` in the current directory.

## Requirements

- [Rust](https://rustup.rs/) (edition 2021, stable)

## Build & Run

```bash
cargo build --release
./target/release/clg
```

Or simply:

```bash
cargo run
```

## Gameplay Loop

1. Enter your character name on first launch.
2. From the **Main Menu** choose: Explore, Farm, Breed Animals, Rest, View Status, Save, Load, or Quit.
3. **Explore** an area to encounter enemies or find treasure.
4. **Farm** — plant crops in empty plots; they complete after a fixed number of seconds and can then be harvested for gold.
5. **Breed Animals** — start a breeding cycle and collect the reward when the timer finishes.
6. Defeat enemies to gain EXP and gold; level up to unlock stronger areas.
