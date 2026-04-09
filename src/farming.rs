use serde::{Deserialize, Serialize};

use crate::world::current_unix_secs;

#[derive(Serialize, Deserialize, Clone)]
pub struct Crop {
    pub name: String,
    pub grow_time_secs: u64,
    pub yield_gold: u32,
    /// Unix timestamp (seconds) when this crop was planted.
    /// Used to check readiness without an external scheduler.
    pub planted_at_secs: Option<u64>,
}

impl Crop {
    /// Returns true once the grow time has elapsed since planting.
    pub fn is_ready(&self) -> bool {
        self.planted_at_secs
            .map(|t| current_unix_secs() >= t + self.grow_time_secs)
            .unwrap_or(false)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Animal {
    pub name: String,
    pub breed_time_secs: u64,
    pub yield_gold: u32,
    pub breeding: bool,
    /// Unix timestamp (seconds) when breeding was started.
    pub breed_started_at_secs: Option<u64>,
}

impl Animal {
    /// Returns true once the breed time has elapsed since breeding began.
    pub fn is_ready(&self) -> bool {
        self.breeding
            && self
                .breed_started_at_secs
                .map(|t| current_unix_secs() >= t + self.breed_time_secs)
                .unwrap_or(false)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Farm {
    pub plots: Vec<Option<Crop>>,
    pub animals: Vec<Animal>,
}

impl Farm {
    pub fn new() -> Self {
        Farm {
            plots: vec![None, None, None, None],
            animals: vec![
                Animal {
                    name: "鸡".to_string(),
                    breed_time_secs: 120,
                    yield_gold: 15,
                    breeding: false,
                    breed_started_at_secs: None,
                },
                Animal {
                    name: "牛".to_string(),
                    breed_time_secs: 300,
                    yield_gold: 50,
                    breeding: false,
                    breed_started_at_secs: None,
                },
                Animal {
                    name: "羊".to_string(),
                    breed_time_secs: 180,
                    yield_gold: 25,
                    breeding: false,
                    breed_started_at_secs: None,
                },
            ],
        }
    }

    pub fn get_crop_types() -> Vec<(String, u64, u32)> {
        vec![
            ("小麦".to_string(), 30, 10),
            ("土豆".to_string(), 60, 25),
            ("胡萝卜".to_string(), 45, 18),
        ]
    }

    /// Plant a crop in the given plot, recording the current time as the
    /// planting timestamp (replaces the old scheduler task).
    pub fn plant(&mut self, plot_idx: usize, crop_type_idx: usize) -> Result<(), String> {
        if plot_idx >= self.plots.len() {
            return Err("无效地块索引。".to_string());
        }
        if self.plots[plot_idx].is_some() {
            return Err("该地块已被占用。".to_string());
        }
        let crop_types = Self::get_crop_types();
        let (name, grow_time, yield_gold) = crop_types
            .get(crop_type_idx)
            .ok_or("无效作物类型。")?
            .clone();

        self.plots[plot_idx] = Some(Crop {
            name,
            grow_time_secs: grow_time,
            yield_gold,
            planted_at_secs: Some(current_unix_secs()),
        });

        Ok(())
    }

    /// Harvest a ready crop, returning its gold value.
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

    /// Start breeding an idle animal, recording the current time.
    pub fn start_breeding(&mut self, animal_idx: usize) -> Result<(), String> {
        let animal = self.animals.get_mut(animal_idx).ok_or("无效动物索引。")?;
        if animal.breeding {
            return Err(format!("{} 已经在繁殖中了。", animal.name));
        }
        animal.breeding = true;
        animal.breed_started_at_secs = Some(current_unix_secs());
        Ok(())
    }

    /// Collect a ready animal's yield, returning its gold value.
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
