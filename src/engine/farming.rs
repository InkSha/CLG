//! Farming and animal breeding subsystem.
//!
//! Time-based mechanics: crops and animals use Unix timestamps to track
//! readiness, checked on demand (no scheduler required).

use serde::{Deserialize, Serialize};

use super::current_unix_secs;

// ── Config types (loaded from data files) ────────────────────────────────────

/// A crop type that can be planted.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CropType {
    pub name: String,
    pub grow_time_secs: u64,
    pub yield_gold: u32,
}

/// An animal type that can be bred.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AnimalType {
    pub name: String,
    pub breed_time_secs: u64,
    pub yield_gold: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Crop {
    pub name: String,
    pub grow_time_secs: u64,
    pub yield_gold: u32,
    pub planted_at_secs: Option<u64>,
}

impl Crop {
    pub fn is_ready(&self) -> bool {
        self.planted_at_secs
            .map(|t| current_unix_secs() >= t + self.grow_time_secs)
            .unwrap_or(false)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Animal {
    pub name: String,
    pub breed_time_secs: u64,
    pub yield_gold: u32,
    pub breeding: bool,
    pub breed_started_at_secs: Option<u64>,
}

impl Animal {
    pub fn is_ready(&self) -> bool {
        self.breeding
            && self
                .breed_started_at_secs
                .map(|t| current_unix_secs() >= t + self.breed_time_secs)
                .unwrap_or(false)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Farm {
    pub plots: Vec<Option<Crop>>,
    pub animals: Vec<Animal>,
}

impl Farm {
    pub fn from_types(animal_types: &[AnimalType], num_plots: usize) -> Self {
        Farm {
            plots: vec![None; num_plots],
            animals: animal_types
                .iter()
                .map(|a| Animal {
                    name: a.name.clone(),
                    breed_time_secs: a.breed_time_secs,
                    yield_gold: a.yield_gold,
                    breeding: false,
                    breed_started_at_secs: None,
                })
                .collect(),
        }
    }

    pub fn default_crop_types() -> Vec<CropType> {
        vec![
            CropType { name: "小麦".into(), grow_time_secs: 30, yield_gold: 10 },
            CropType { name: "土豆".into(), grow_time_secs: 60, yield_gold: 25 },
            CropType { name: "胡萝卜".into(), grow_time_secs: 45, yield_gold: 18 },
        ]
    }

    pub fn default_animal_types() -> Vec<AnimalType> {
        vec![
            AnimalType { name: "鸡".into(), breed_time_secs: 120, yield_gold: 15 },
            AnimalType { name: "牛".into(), breed_time_secs: 300, yield_gold: 50 },
            AnimalType { name: "羊".into(), breed_time_secs: 180, yield_gold: 25 },
        ]
    }

    pub fn plant(&mut self, plot_idx: usize, crop: &CropType) -> Result<(), String> {
        if plot_idx >= self.plots.len() {
            return Err("无效地块索引。".into());
        }
        if self.plots[plot_idx].is_some() {
            return Err("该地块已被占用。".into());
        }
        self.plots[plot_idx] = Some(Crop {
            name: crop.name.clone(),
            grow_time_secs: crop.grow_time_secs,
            yield_gold: crop.yield_gold,
            planted_at_secs: Some(current_unix_secs()),
        });
        Ok(())
    }

    pub fn harvest(&mut self, plot_idx: usize) -> Option<u32> {
        if plot_idx >= self.plots.len() {
            return None;
        }
        let ready = self.plots[plot_idx].as_ref()?.is_ready();
        if ready {
            let gold = self.plots[plot_idx].as_ref().unwrap().yield_gold;
            self.plots[plot_idx] = None;
            Some(gold)
        } else {
            None
        }
    }

    pub fn start_breeding(&mut self, animal_idx: usize) -> Result<(), String> {
        let animal = self.animals.get_mut(animal_idx).ok_or("无效动物索引。")?;
        if animal.breeding {
            return Err(format!("{} 已经在繁殖中了。", animal.name));
        }
        animal.breeding = true;
        animal.breed_started_at_secs = Some(current_unix_secs());
        Ok(())
    }

    pub fn collect_animal(&mut self, animal_idx: usize) -> Option<u32> {
        let animal = self.animals.get_mut(animal_idx)?;
        if !animal.is_ready() {
            return None;
        }
        let gold = animal.yield_gold;
        animal.breeding = false;
        animal.breed_started_at_secs = None;
        Some(gold)
    }
}
