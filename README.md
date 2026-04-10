# CLG
Command Line Game

A terminal simulation/management RPG written in Rust. Explore dangerous areas, battle enemies, manage a farm, breed animals, and strengthen your character — all driven by editable data files.

## Features

- **File-driven Configuration** — Areas, crop types, and animal types are all defined in plain YAML files under `world/config/`. Edit them to customise the game world without touching Rust code.
- **Action System** — A global `world/action.yaml` (plus optional per-area overrides) maps human-readable commands to built-in operations. The main game loop reads commands you type, matches them against the action map, and executes the corresponding built-in.
- **Built-in Commands** — A set of Linux-inspired built-ins (`ls`, `cd`, `cat`, `echo`, `grep`, `save`, `quit`, …) form the primitive vocabulary of the game.
- **Exploration** — Areas defined in `world/config/areas.yaml` (default: five tiers from Forest → Dragon's Peak). Entering a new area triggers an encounter.
- **Combat** — Turn-based battles against enemies scaled to the area's level. Attack or attempt to flee.
- **Farming** — Crop types in `world/config/crops.yaml`. Plant in farm plots; crops grow in real time and can be harvested for gold.
- **Breeding** — Animal types in `world/config/animals.yaml`. Start a breeding cycle and collect the gold reward when the timer finishes.
- **Character Progression** — Gain EXP and gold from combat/exploration. Level up to increase HP, ATK, and DEF.
- **Filesystem-driven World** — All game state (player, farm plots, animals, areas) is persisted as **YAML** files under `world/`.
- **Entity Templates** — Drop a `*.template.yaml` file anywhere inside `world/` to procedurally generate entity YAML files from a schema.
- **Real-time Watcher** — A background `notify` watcher surfaces filesystem events (file creation, removal, player movement, template changes) in the UI.

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
2. The main screen shows your current area, the files inside it, and all available commands read from `world/action.yaml`.
3. Type a command and press **Enter**.
4. Use `explor <area>` (or `cd <area>` if you added it) to travel to another area (triggers an encounter).
5. Defeat enemies to gain EXP and gold; level up to unlock stronger areas.
6. Use `farm` to plant/harvest crops, `breed` to manage animals — add those lines to `world/action.yaml` first.
7. Use `save` to persist your progress, `exit` (or `quit`) to exit.

### Example session

```
📍 当前位置：森林  (world/森林/)
──────────────────────────────────────────────────
📂 world/森林/
   area.yaml
   player.yaml
──────────────────────────────────────────────────
📋 可用指令：
  back                  cd ..
  explor                cd $1

> explor 黑暗洞穴
```

## World Directory Layout

```
world/
├── config/
│   ├── areas.yaml         ← area definitions (editable)
│   ├── crops.yaml         ← crop type definitions (editable)
│   └── animals.yaml       ← animal type definitions (editable)
├── action.yaml            ← global action map (editable)
├── 森林/
│   ├── area.yaml          ← area metadata
│   ├── action.yaml        ← area-specific action overrides (optional)
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

## Configuration Files

### `world/action.yaml` — Action Map

Maps user-typed commands to built-in operations. Keys may contain spaces; values are built-in command strings. `$1`, `$2`, … are replaced with the arguments the player provides.

Only **`explor`** and **`back`** are active by default. Copy any of the entries below into `world/action.yaml` to unlock the corresponding Linux-style commands.

```yaml
# --- navigation (always present) ---
explor: cd $1          # enter an area
back: cd ..            # return home

# --- common Linux ops commands (add as needed) ---
ls: ls .               # list current area
ls $1: ls $1           # list named area
cd: cd $1              # alias for explor
pwd: status            # show current location / player stats
cat: cat $1            # display a YAML entity file
grep: grep $1          # search file names & content
echo $1 > $2: echo $1 > $2  # write text to a file
ps: status             # show player stats (process-list metaphor)
top: status            # show player stats (top metaphor)
df: status             # show player stats (disk-free metaphor)
save: save             # persist game state
exit: quit             # exit the game
farm: farm             # open farming sub-menu
breed: breed           # open animal-breeding sub-menu
rest: rest             # recover HP
```

Per-area overrides live in `world/<area>/action.yaml` and are merged on top of the global map (area keys win).

### Built-in Commands

The following built-ins are always available in the engine. Add the corresponding entry to `world/action.yaml` to expose them under a Linux-style command name.

| Built-in            | Suggested action.yaml entry         | Description                                         |
|---------------------|--------------------------------------|-----------------------------------------------------|
| `ls [path]`         | `ls: ls .` / `ls $1: ls $1`         | List entity files in the current (or named) area    |
| `cd <area>`         | `cd: cd $1` / `explor: cd $1`       | Move to a named area and trigger an exploration encounter |
| `cd ~`              | *(navigate home manually)*           | Return home without triggering an encounter         |
| `cd ..`             | `back: cd ..`                        | Return to the start area (treated as home, not parent navigation) |
| `cat <file>`        | `cat: cat $1`                        | Read and display a YAML entity file                 |
| `echo <text> > <f>` | `echo $1 > $2: echo $1 > $2`        | Write text to a file in the current area            |
| `grep <pattern>`    | `grep: grep $1`                      | Search file names and content in the current area   |
| `status`            | `ps: status` / `pwd: status`         | Display player stats and area list                  |
| `farm`              | `farm: farm`                         | Open the farming sub-menu                           |
| `breed`             | `breed: breed`                       | Open the animal breeding sub-menu                   |
| `rest`              | `rest: rest`                         | Rest and recover HP (costs 20 gold → +30 HP)        |
| `save`              | `save: save`                         | Save game state to disk                             |
| `quit`              | `exit: quit`                         | Exit the game                                       |

### `world/config/areas.yaml` — Area Definitions

```yaml
- name: 森林
  description: 宁静的树林，适合新手探索。
  level_req: 1
  enemy_level: 1
  explore_cost_hp: 0
- name: 黑暗洞穴
  description: 潮湿而充满危险的地下洞穴。
  level_req: 3
  enemy_level: 3
  explore_cost_hp: 2
# … add more areas here
```

### `world/config/crops.yaml` — Crop Types

```yaml
- name: 小麦
  grow_time_secs: 30
  yield_gold: 10
- name: 土豆
  grow_time_secs: 60
  yield_gold: 25
# … add more crops here
```

### `world/config/animals.yaml` — Animal Types

```yaml
- name: 鸡
  breed_time_secs: 120
  yield_gold: 15
- name: 牛
  breed_time_secs: 300
  yield_gold: 50
# … add more animals here
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
