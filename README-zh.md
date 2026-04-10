# CLG
命令行游戏

一款用 Rust 编写的终端模拟/经营类 RPG 游戏。探索危险地区、与敌人战斗、经营农场、繁殖动物，不断强化你的角色。

## 功能特性

- **探索** — 五个分级区域（森林 → 黑暗洞穴 → 鬼魂废墟 → 火山荒地 → 龙之巅峰），每个区域都有更高的等级要求和更强大的敌人。
- **战斗** — 与每个区域中按等级缩放的 5 种敌人进行回合制战斗。可选择攻击或尝试逃跑。
- **农场** — 在 4 块农田中种植小麦、土豆或胡萝卜。作物通过后台调度线程实时生长。
- **繁殖** — 饲养鸡、牛和羊。开始繁殖并在计时器结束后收取金币奖励。
- **角色成长** — 通过战斗和探索获取经验和金币。升级后可提升生命值、攻击力和防御力。
- **休息** — 随时花费 20 金币回复 30 点生命值。
- **文件系统驱动的世界** — 所有游戏状态（角色、农场地块、动物、区域）均以 **YAML** 格式持久化保存至 `world/` 目录下。
- **实体模板** — 在 `world/` 的任意子目录中放置 `*.template.yaml` 文件，即可按照 schema 自动生成实体 YAML 文件。

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
2. 在**主菜单**中选择：探索、农场、繁殖动物、休息、查看状态、保存游戏、读取游戏或退出。
3. **探索**区域以遭遇敌人或发现宝藏。
4. **农场** — 在空地块中种植作物；作物在固定秒数后成熟，之后可收获以换取金币。
5. **繁殖动物** — 开始一轮繁殖并在计时器结束后收取奖励。
6. 击败敌人以获取经验和金币；升级后可解锁更强的区域。

## 世界目录结构

```
world/
├── 森林/
│   ├── area.yaml          ← 区域元数据
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
