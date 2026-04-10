//! Entity system.
//!
//! Every game object (monster, NPC, item, crop, animal) is an [`Entity`] with
//! a unique [`EntityId`] and a bag of [`Value`] properties.  The entity itself
//! is the **true state**; the virtual filesystem merely exposes an interface
//! view of it.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Unique numeric handle for an entity.
pub type EntityId = u64;

/// What kind of thing an entity represents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EntityKind {
    Monster,
    Npc,
    Item,
    Crop,
    Animal,
}

impl std::fmt::Display for EntityKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityKind::Monster => write!(f, "怪物"),
            EntityKind::Npc => write!(f, "NPC"),
            EntityKind::Item => write!(f, "物品"),
            EntityKind::Crop => write!(f, "作物"),
            EntityKind::Animal => write!(f, "动物"),
        }
    }
}

/// A dynamically-typed value stored in entity state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(v) => write!(f, "{}", v),
            Value::Float(v) => write!(f, "{:.1}", v),
            Value::Str(v) => write!(f, "{}", v),
            Value::Bool(v) => {
                if *v {
                    write!(f, "是")
                } else {
                    write!(f, "否")
                }
            }
        }
    }
}

/// A game-world entity.
///
/// This is the **true state** of a game object.  The virtual filesystem
/// presents a read-only (or controlled-write) view of it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub kind: EntityKind,
    pub name: String,
    /// Which area this entity currently resides in.
    pub area: String,
    /// Arbitrary key→value properties (HP, attack, description, …).
    pub state: HashMap<String, Value>,
}

#[allow(dead_code)]
impl Entity {
    /// Read an integer property, returning 0 if missing or wrong type.
    pub fn get_int(&self, key: &str) -> i64 {
        match self.state.get(key) {
            Some(Value::Int(v)) => *v,
            _ => 0,
        }
    }

    /// Read a string property, returning `""` if missing or wrong type.
    pub fn get_str(&self, key: &str) -> &str {
        match self.state.get(key) {
            Some(Value::Str(v)) => v.as_str(),
            _ => "",
        }
    }

    /// Read a bool property, returning `false` if missing or wrong type.
    pub fn get_bool(&self, key: &str) -> bool {
        match self.state.get(key) {
            Some(Value::Bool(v)) => *v,
            _ => false,
        }
    }

    /// Set an integer property.
    pub fn set_int(&mut self, key: &str, value: i64) {
        self.state.insert(key.to_string(), Value::Int(value));
    }

    /// Set a string property.
    pub fn set_str(&mut self, key: &str, value: &str) {
        self.state
            .insert(key.to_string(), Value::Str(value.to_string()));
    }

    /// Set a bool property.
    pub fn set_bool(&mut self, key: &str, value: bool) {
        self.state.insert(key.to_string(), Value::Bool(value));
    }

    /// Render entity info as human-readable text (the "file interface").
    pub fn to_display(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("[{}] {}\n", self.kind, self.name));

        // Display properties in a consistent order.
        let mut keys: Vec<&String> = self.state.keys().collect();
        keys.sort();
        for key in keys {
            let val = &self.state[key];
            out.push_str(&format!("{}: {}\n", key, val));
        }
        out
    }
}
