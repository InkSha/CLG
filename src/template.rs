//! Entity template system.
//!
//! A *template file* is a JSON file whose name ends with `.template.json`.
//! Placing such a file in any directory causes the game to generate a concrete
//! entity JSON file (named by the `output` field) **in that same directory**.
//! Templates are scoped to their own folder and its subdirectories: they never
//! generate entities outside the directory tree in which they reside.
//!
//! # Template file format
//! ```json
//! {
//!   "entity":  "enemy",
//!   "output":  "enemy_generated.json",
//!   "schema": {
//!     "name":        { "type": "string",  "format": "enemy_name" },
//!     "hp":          { "type": "integer", "range": [20, 80] },
//!     "max_hp":      { "type": "integer", "range": [20, 80] },
//!     "attack":      { "type": "integer", "range": [5, 20]  },
//!     "defense":     { "type": "integer", "range": [1, 8]   },
//!     "exp_reward":  { "type": "integer", "range": [10, 40] },
//!     "gold_reward": { "type": "integer", "range": [3, 15]  },
//!     "skills":      { "type": "array", "length": [0, 3],
//!                      "items": [{ "type": "string" }] }
//!   }
//! }
//! ```
//!
//! # Supported field types
//!
//! | `type`    | Extra keys                                  |
//! |-----------|---------------------------------------------|
//! | `string`  | `length`, `format`, `value`                 |
//! | `integer` | `range`, `value`                            |
//! | `float`   | `range`, `value`                            |
//! | `boolean` | `value`                                     |
//! | `array`   | `length` (required), `items` (required)     |
//! | `object`  | `fields` (required)                         |

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rand::Rng;
use serde::{Deserialize, Serialize};

// ── String format vocabulary ──────────────────────────────────────────────────

/// Named format hints that guide meaningful string generation.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum StringFormat {
    /// Chinese personal name (surname + given-name character).
    Name,
    /// Enemy / monster name drawn from a pre-defined list.
    EnemyName,
    /// Area / location name.
    AreaName,
    /// Short area description sentence.
    Description,
    /// Crop / plant name.
    CropName,
    /// Domestic animal name.
    AnimalName,
    /// Generic single word.
    Word,
}

// ── Field schema ──────────────────────────────────────────────────────────────

/// Schema that describes how to generate a single JSON field value.
///
/// The `"type"` key in the JSON document selects the enum variant.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FieldSchema {
    /// A UTF-8 string field.
    String {
        /// Optional character-length range `[min, max]`.
        #[serde(skip_serializing_if = "Option::is_none")]
        length: Option<[usize; 2]>,
        /// Optional named format for generating meaningful strings.
        #[serde(skip_serializing_if = "Option::is_none")]
        format: Option<StringFormat>,
        /// Optional fixed value; skips random generation when set.
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<String>,
    },

    /// A 64-bit signed integer field.
    Integer {
        /// Optional inclusive range `[min, max]`.
        #[serde(skip_serializing_if = "Option::is_none")]
        range: Option<[i64; 2]>,
        /// Optional fixed value.
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<i64>,
    },

    /// A 64-bit floating-point field.
    Float {
        /// Optional inclusive range `[min, max]`.
        #[serde(skip_serializing_if = "Option::is_none")]
        range: Option<[f64; 2]>,
        /// Optional fixed value.
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<f64>,
    },

    /// A boolean field.
    Boolean {
        /// Optional fixed value; randomised if absent.
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<bool>,
    },

    /// A JSON array field.
    Array {
        /// Inclusive element-count range `[min, max]`.
        length: [usize; 2],
        /// Item schemas; cycled when fewer schemas than generated items.
        items: Vec<FieldSchema>,
    },

    /// A nested JSON object field.
    Object {
        /// Nested field schemas keyed by field name.
        fields: HashMap<String, FieldSchema>,
    },
}

// ── Entity template ───────────────────────────────────────────────────────────

/// Parsed contents of a `*.template.json` file.
#[derive(Serialize, Deserialize, Debug)]
pub struct EntityTemplate {
    /// Logical entity type label (informational, e.g. `"player"`, `"enemy"`).
    pub entity: String,
    /// Output filename relative to the template file's own directory.
    pub output: String,
    /// Top-level field schemas keyed by field name.
    pub schema: HashMap<String, FieldSchema>,
}

// ── Data tables ───────────────────────────────────────────────────────────────

static SURNAMES: &[&str] = &[
    "李", "王", "张", "刘", "陈", "杨", "赵", "黄", "周", "吴",
    "徐", "孙", "胡", "朱", "高", "林", "何", "郭", "马", "罗",
];

static GIVEN_CHARS: &[&str] = &[
    "伟", "芳", "娜", "敏", "静", "丽", "强", "磊", "洋", "艳",
    "勇", "军", "杰", "涛", "明", "超", "霞", "平", "刚", "华",
    "辉", "玲", "金", "蕾", "峰", "婷", "鑫", "宇", "浩", "龙",
];

static ENEMY_NAMES: &[&str] = &[
    "哥布林", "狼人", "强盗头目", "石像鬼", "黑暗法师",
    "骷髅战士", "食人魔", "巨蜘蛛", "蝙蝠王", "暗影杀手",
    "炎魔", "冰龙", "毒蛇精", "沙漠蝎王", "古树精",
];

static AREA_NAMES: &[&str] = &[
    "幽暗森林", "魔法峡谷", "碎骨荒原", "冰封雪山", "熔岩洞窟",
    "幽灵废墟", "迷雾沼泽", "雷鸣高地", "深海神庙", "星落平原",
];

static AREA_DESCRIPTIONS: &[&str] = &[
    "充满神秘气息的古老地带。",
    "危机四伏，只有强者才能生存。",
    "传说中曾有英雄在此留下足迹。",
    "空气中弥漫着淡淡的魔法能量。",
    "荒无人烟，却隐藏着无数宝藏。",
];

static CROP_NAMES: &[&str] = &[
    "小麦", "土豆", "胡萝卜", "南瓜", "番茄",
    "草莓", "玉米", "西瓜", "甘蔗", "甜菜",
];

static ANIMAL_NAMES: &[&str] = &[
    "鸡", "牛", "羊", "猪", "兔",
    "马", "鸭", "鹅", "驴", "骆驼",
];

static WORDS: &[&str] = &[
    "勇气", "智慧", "力量", "速度", "敏捷",
    "耐力", "魔力", "神秘", "古老", "传奇",
];

// ── Value generation ──────────────────────────────────────────────────────────

/// Generate a random [`serde_json::Value`] that satisfies `schema`.
pub fn generate_value(schema: &FieldSchema) -> serde_json::Value {
    let mut rng = rand::thread_rng();
    match schema {
        FieldSchema::String { length, format, value } => {
            if let Some(v) = value {
                return serde_json::Value::String(v.clone());
            }
            let mut s = match format {
                Some(StringFormat::Name) => gen_name(&mut rng),
                Some(StringFormat::EnemyName) => pick(&mut rng, ENEMY_NAMES).to_string(),
                Some(StringFormat::AreaName) => pick(&mut rng, AREA_NAMES).to_string(),
                Some(StringFormat::Description) => pick(&mut rng, AREA_DESCRIPTIONS).to_string(),
                Some(StringFormat::CropName) => pick(&mut rng, CROP_NAMES).to_string(),
                Some(StringFormat::AnimalName) => pick(&mut rng, ANIMAL_NAMES).to_string(),
                Some(StringFormat::Word) | None => pick(&mut rng, WORDS).to_string(),
            };
            if let Some([min, max]) = length {
                let target = rng.gen_range(*min..=(*max).max(*min));
                s = s.chars().take(target).collect();
            }
            serde_json::Value::String(s)
        }

        FieldSchema::Integer { range, value } => {
            if let Some(v) = value {
                return serde_json::json!(*v);
            }
            let n = if let Some([min, max]) = range {
                rng.gen_range(*min..=(*max).max(*min))
            } else {
                0
            };
            serde_json::json!(n)
        }

        FieldSchema::Float { range, value } => {
            if let Some(v) = value {
                return serde_json::json!(*v);
            }
            let f = if let Some([min, max]) = range {
                rng.gen_range(*min..=(*max).max(*min))
            } else {
                0.0
            };
            serde_json::json!(f)
        }

        FieldSchema::Boolean { value } => {
            let b = value.unwrap_or_else(|| rand::thread_rng().gen_bool(0.5));
            serde_json::json!(b)
        }

        FieldSchema::Array { length: [min, max], items } => {
            if items.is_empty() {
                return serde_json::json!([]);
            }
            let count = rng.gen_range(*min..=(*max).max(*min));
            let arr: Vec<serde_json::Value> = (0..count)
                .map(|i| generate_value(&items[i % items.len()]))
                .collect();
            serde_json::json!(arr)
        }

        FieldSchema::Object { fields } => {
            serde_json::Value::Object(generate_object(fields))
        }
    }
}

/// Generate a full entity JSON object from a top-level schema map.
pub fn generate_object(
    schema: &HashMap<String, FieldSchema>,
) -> serde_json::Map<String, serde_json::Value> {
    let mut obj = serde_json::Map::new();
    for (key, field_schema) in schema {
        obj.insert(key.clone(), generate_value(field_schema));
    }
    obj
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn gen_name(rng: &mut impl Rng) -> String {
    let surname = pick(rng, SURNAMES);
    let given = pick(rng, GIVEN_CHARS);
    format!("{}{}", surname, given)
}

fn pick<'a>(rng: &mut impl Rng, list: &[&'a str]) -> &'a str {
    list[rng.gen_range(0..list.len())]
}

// ── File I/O ──────────────────────────────────────────────────────────────────

/// Parse an [`EntityTemplate`] from a `*.template.json` file on disk.
pub fn load_template(path: &Path) -> Result<EntityTemplate, String> {
    let json = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&json)
        .map_err(|e| format!("解析模板 {} 失败: {}", path.display(), e))
}

/// Recursively find all `*.template.json` files under `root`.
pub fn find_templates(root: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return result;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            result.extend(find_templates(&path));
        } else if path
            .file_name()
            .and_then(|n| n.to_str())
            .map(is_template_filename)
            .unwrap_or(false)
        {
            result.push(path);
        }
    }
    result
}

/// Returns `true` when `filename` follows the `*.template.json` convention.
pub fn is_template_filename(filename: &str) -> bool {
    filename.ends_with(".template.json")
}

/// Apply `template` by generating its output file inside `template_path`'s
/// parent directory.  Returns the path of the written file.
///
/// If the output file already exists and `overwrite` is `false` the function
/// returns the existing path without modifying it.
pub fn apply_template(
    template_path: &Path,
    template: &EntityTemplate,
    overwrite: bool,
) -> Result<PathBuf, String> {
    let dir = template_path
        .parent()
        .ok_or_else(|| "模板路径无效".to_string())?;
    let output_path = dir.join(&template.output);
    if output_path.exists() && !overwrite {
        return Ok(output_path);
    }
    let obj = generate_object(&template.schema);
    let json = serde_json::to_string_pretty(&serde_json::Value::Object(obj))
        .map_err(|e| e.to_string())?;
    std::fs::write(&output_path, json).map_err(|e| e.to_string())?;
    Ok(output_path)
}

// ── Built-in template definitions ────────────────────────────────────────────

/// Return the canonical template JSON string for each built-in entity type.
///
/// These templates cover every entity that the game manages:
/// `player`, `enemy`, `area`, `crop`, and `animal`.
pub fn builtin_templates() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "player.template.json",
            r#"{
  "entity": "player",
  "output": "player_generated.json",
  "schema": {
    "name":        { "type": "string",  "format": "name" },
    "hp":          { "type": "integer", "value": 100 },
    "max_hp":      { "type": "integer", "value": 100 },
    "attack":      { "type": "integer", "range": [8, 15] },
    "defense":     { "type": "integer", "range": [3, 8] },
    "level":       { "type": "integer", "value": 1 },
    "exp":         { "type": "integer", "value": 0 },
    "exp_to_next": { "type": "integer", "value": 100 },
    "gold":        { "type": "integer", "range": [20, 80] }
  }
}"#,
        ),
        (
            "enemy.template.json",
            r#"{
  "entity": "enemy",
  "output": "enemy_generated.json",
  "schema": {
    "name":        { "type": "string",  "format": "enemy_name" },
    "hp":          { "type": "integer", "range": [20, 80] },
    "max_hp":      { "type": "integer", "range": [20, 80] },
    "attack":      { "type": "integer", "range": [5, 20] },
    "defense":     { "type": "integer", "range": [1, 8] },
    "exp_reward":  { "type": "integer", "range": [10, 40] },
    "gold_reward": { "type": "integer", "range": [3, 15] },
    "skills":      { "type": "array", "length": [0, 5],
                     "items": [{ "type": "string" }] }
  }
}"#,
        ),
        (
            "area.template.json",
            r#"{
  "entity": "area",
  "output": "area_generated.json",
  "schema": {
    "name":            { "type": "string",  "format": "area_name" },
    "description":     { "type": "string",  "format": "description" },
    "level_req":       { "type": "integer", "range": [1, 12] },
    "enemy_level":     { "type": "integer", "range": [1, 12] },
    "explore_cost_hp": { "type": "integer", "range": [0, 15] }
  }
}"#,
        ),
        (
            "crop.template.json",
            r#"{
  "entity": "crop",
  "output": "crop_generated.json",
  "schema": {
    "status":         { "type": "string",  "value": "occupied" },
    "crop_name":      { "type": "string",  "format": "crop_name" },
    "grow_time_secs": { "type": "integer", "range": [30, 120] },
    "yield_gold":     { "type": "integer", "range": [10, 50] },
    "planted_at_secs":{ "type": "integer", "value": 0 }
  }
}"#,
        ),
        (
            "animal.template.json",
            r#"{
  "entity": "animal",
  "output": "animal_generated.json",
  "schema": {
    "name":                 { "type": "string",  "format": "animal_name" },
    "breed_time_secs":      { "type": "integer", "range": [60, 360] },
    "yield_gold":           { "type": "integer", "range": [10, 60] },
    "breeding":             { "type": "boolean", "value": false },
    "breed_started_at_secs":{ "type": "integer", "value": 0 }
  }
}"#,
        ),
    ]
}
