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
- **Filesystem-driven World** — All game state (player, farm plots, animals, areas) is persisted as **YAML** files under the `world/` directory.
- **Entity Templates** — Drop a `*.template.yaml` file anywhere inside `world/` to procedurally generate entity YAML files from a schema.

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

## World Directory Layout

```
world/
├── 森林/
│   ├── area.yaml          ← area metadata
│   └── player.yaml        ← player data (present when player is here)
├── 黑暗洞穴/
│   └── area.yaml
├── 农场/
│   ├── plot_0.yaml        ← farm plot state
│   ├── plot_1.yaml
│   ├── 鸡.yaml            ← animal state
│   └── …
└── templates/
    ├── player.template.yaml   ← built-in entity templates
    ├── enemy.template.yaml
    ├── area.template.yaml
    ├── crop.template.yaml
    └── animal.template.yaml
```

## Entity Templates

Templates let you define how entity YAML files are generated without touching Rust code. Place a `*.template.yaml` file in any subdirectory of `world/`; on startup the game automatically generates the `output` file in the same directory.

Example `enemy.template.yaml`:

```yaml
entity: enemy
output: enemy_generated.yaml
schema:
  name:
    type: string
    format: enemy_name
  hp:
    type: integer
    range: [20, 80]
  skills:
    type: array
    length: [0, 5]
    items:
      - type: string
```

### Supported field types

| `type`    | Extra keys                              |
|-----------|-----------------------------------------|
| `string`  | `length`, `format`, `value`             |
| `integer` | `range`, `value`                        |
| `float`   | `range`, `value`                        |
| `boolean` | `value`                                 |
| `array`   | `length` (required), `items` (required) |
| `object`  | `fields` (required)                     |

String `format` options: `name` (Chinese personal name), `enemy_name`, `area_name`, `description`, `crop_name`, `animal_name`, `word`.
