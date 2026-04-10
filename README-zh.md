# CLG
命令行游戏

一款用 Rust 编写的终端模拟/经营类 RPG 游戏。探索危险地区、与敌人战斗、经营农场、繁殖动物，不断强化你的角色——所有配置均由可编辑的数据文件驱动。

## 功能特性

- **文件驱动配置** — 区域、作物类型和动物类型均定义于 `world/config/` 下的 YAML 文件中，无需修改 Rust 代码即可自定义游戏世界。
- **行动系统** — 全局 `world/action.yaml`（以及可选的区域级覆盖文件）将可读命令映射到内置操作。主游戏循环读取你输入的命令，匹配行动映射，并执行对应的内置指令。
- **内置指令** — 一系列 Linux 风格的内置指令（`ls`、`cd`、`cat`、`echo`、`grep`、`save`、`quit`……）构成游戏的基本词汇。
- **探索** — 区域定义于 `world/config/areas.yaml`（默认：从森林到龙之巅峰共五个等级区域）。进入新区域会触发遭遇事件。
- **战斗** — 与按区域等级缩放的敌人进行回合制战斗，可选择攻击或逃跑。
- **农场** — 作物类型定义于 `world/config/crops.yaml`，在农田中种植作物，实时生长后可收获金币。
- **繁殖** — 动物类型定义于 `world/config/animals.yaml`，开始繁殖并在计时结束后收取金币奖励。
- **角色成长** — 通过战斗和探索获取经验和金币，升级后提升生命值、攻击力和防御力。
- **文件系统驱动的世界** — 所有游戏状态均以 **YAML** 格式持久化保存至 `world/` 目录下。
- **实体模板** — 在 `world/` 的任意子目录中放置 `*.template.yaml` 文件，游戏启动时会自动按 schema 生成对应的实体 YAML 文件。
- **实时文件监听** — 后台 `notify` 监听器将文件系统事件（文件创建、删除、玩家移动、模板变更）实时显示在界面中。

## 环境要求

- [Rust](https://rustup.rs/)（2021 版，稳定版）

## 构建与运行

```bash
cargo build --release
./target/release/clg
```

或直接运行：

```bash
cargo run
```

## 游戏流程

1. 首次启动时输入角色名。
2. 主界面显示当前区域、区域内文件，以及从 `world/action.yaml` 读取的全部可用指令。
3. 输入指令并按 **Enter**。
4. 使用 `go <区域>` 或 `explor <区域>` 前往其他区域（触发遭遇事件）。
5. 击败敌人获取经验和金币，升级解锁更强区域。
6. 使用 `farm` 管理农场，`breed` 管理动物繁殖。
7. 使用 `save` 保存进度，`quit` 退出游戏。

### 示例游戏会话

```
📍 当前位置：森林  (world/森林/)
──────────────────────────────────────────────────
📂 world/森林/
   area.yaml
   player.yaml
──────────────────────────────────────────────────
📋 可用指令：
  back                  cd ..
  breed                 breed
  explor                cd $1
  farm                  farm
  find                  grep $1
  go                    cd $1
  go home               cd ~
  open world            ls .
  quit                  quit
  read                  cat $1
  rest                  rest
  save                  save
  status                status

> go 黑暗洞穴
```

## 世界目录结构

```
world/
├── config/
│   ├── areas.yaml         ← 区域定义（可编辑）
│   ├── crops.yaml         ← 作物类型定义（可编辑）
│   └── animals.yaml       ← 动物类型定义（可编辑）
├── action.yaml            ← 全局行动映射（可编辑）
├── 森林/
│   ├── area.yaml          ← 区域元数据
│   ├── action.yaml        ← 区域专属行动覆盖（可选）
│   └── player.yaml        ← 玩家数据（玩家在此区域时存在）
├── 黑暗洞穴/
│   └── area.yaml
├── 农场/
│   ├── plot_0.yaml        ← 农场地块状态
│   ├── plot_1.yaml
│   ├── 鸡.yaml            ← 动物状态
│   └── …
└── templates/
    ├── player.template.yaml   ← 内置实体模板
    ├── enemy.template.yaml
    ├── area.template.yaml
    ├── crop.template.yaml
    └── animal.template.yaml
```

## 配置文件

### `world/action.yaml` — 行动映射

将玩家输入的指令映射到内置操作。键可以包含空格；值为内置指令字符串，`$1`、`$2`…… 会被玩家输入的实际参数替换。

```yaml
open world: ls .
explor: cd $1
go home: cd ~
quit: quit
save: save
read: cat $1
back: cd ..
go: cd $1
find: grep $1
farm: farm
breed: breed
rest: rest
status: status
```

区域专属覆盖存放于 `world/<区域>/action.yaml`，会合并到全局映射之上（区域键优先）。

### 内置指令

| 内置指令             | 说明                                          |
|---------------------|-----------------------------------------------|
| `ls [路径]`          | 列出当前（或指定）区域的实体文件              |
| `cd <区域>`          | 移动到指定区域并触发探索遭遇                  |
| `cd ~`               | 回到起始区域，不触发遭遇                      |
| `cd ..`              | 回到起始区域，不触发遭遇                      |
| `cat <文件>`         | 读取并显示 YAML 实体文件内容                  |
| `echo <内容> > <f>`  | 向当前区域的文件写入文本内容                  |
| `grep <关键词>`      | 在当前区域的文件名和内容中搜索                |
| `farm`               | 打开农场子菜单                                |
| `breed`              | 打开动物繁殖子菜单                            |
| `rest`               | 休息并恢复生命值（花费 20 金币 → 回复 30 HP） |
| `status`             | 显示玩家属性和区域列表                        |
| `save`               | 保存游戏状态到磁盘                            |
| `quit`               | 退出游戏                                      |

### `world/config/areas.yaml` — 区域定义

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
# … 在此添加更多区域
```

### `world/config/crops.yaml` — 作物类型

```yaml
- name: 小麦
  grow_time_secs: 30
  yield_gold: 10
- name: 土豆
  grow_time_secs: 60
  yield_gold: 25
# … 在此添加更多作物
```

### `world/config/animals.yaml` — 动物类型

```yaml
- name: 鸡
  breed_time_secs: 120
  yield_gold: 15
- name: 牛
  breed_time_secs: 300
  yield_gold: 50
# … 在此添加更多动物
```

## 实体模板

模板允许你通过配置定义实体 YAML 文件的生成方式，无需修改 Rust 代码。将 `*.template.yaml` 文件放入 `world/` 的任意子目录，游戏启动时会自动在同目录下生成对应的 `output` 文件。

示例 `enemy.template.yaml`：

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

### 支持的字段类型

| `type`    | 额外配置项                              |
|-----------|-----------------------------------------|
| `string`  | `length`、`format`、`value`             |
| `integer` | `range`、`value`                        |
| `float`   | `range`、`value`                        |
| `boolean` | `value`                                 |
| `array`   | `length`（必填）、`items`（必填）       |
| `object`  | `fields`（必填）                        |

`string` 的 `format` 可选值：`name`（中文人名）、`enemy_name`（敌人名）、`area_name`（地区名）、`description`（描述句）、`crop_name`（作物名）、`animal_name`（动物名）、`word`（通用词汇）。
