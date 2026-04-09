use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Returns the current time as Unix seconds.
fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// A crop growing in a farm plot.
/// Readiness is determined by comparing the current wall-clock time against
/// `planted_at_unix + grow_time_secs`, so the state persists across restarts.
#[derive(Serialize, Deserialize, Clone)]
pub struct Crop {
    pub name: String,
    pub grow_time_secs: u64,
    pub yield_gold: u32,
    /// Unix timestamp (seconds) when the crop was planted; `None` = not yet planted.
    pub planted_at_unix: Option<u64>,
}

impl Crop {
    pub fn is_ready(&self) -> bool {
        match self.planted_at_unix {
            Some(planted) => now_unix() >= planted + self.grow_time_secs,
            None => false,
        }
    }
}

/// A farm animal that can be put into breeding.
/// Readiness is determined by comparing the current wall-clock time against
/// `breeding_start_unix + breed_time_secs`.
#[derive(Serialize, Deserialize, Clone)]
pub struct Animal {
    pub name: String,
    pub breed_time_secs: u64,
    pub yield_gold: u32,
    /// Unix timestamp (seconds) when breeding started; `None` = idle.
    pub breeding_start_unix: Option<u64>,
    pub breeding: bool,
}

impl Animal {
    pub fn is_ready(&self) -> bool {
        match self.breeding_start_unix {
            Some(start) => self.breeding && now_unix() >= start + self.breed_time_secs,
            None => false,
        }
    }
}

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
                    breeding_start_unix: None,
                    breeding: false,
                },
                Animal {
                    name: "牛".to_string(),
                    breed_time_secs: 300,
                    yield_gold: 50,
                    breeding_start_unix: None,
                    breeding: false,
                },
                Animal {
                    name: "羊".to_string(),
                    breed_time_secs: 180,
                    yield_gold: 25,
                    breeding_start_unix: None,
                    breeding: false,
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

    /// Plant a crop in a plot. Records the current Unix time as `planted_at_unix`.
    pub fn plant(
        &mut self,
        plot_idx: usize,
        crop_type_idx: usize,
    ) -> Result<(), String> {
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
            planted_at_unix: Some(now_unix()),
        });
        Ok(())
    }

    /// Harvest a ready crop. Returns the gold earned, or `None` if not ready.
    pub fn harvest(&mut self, plot_idx: usize) -> Option<u32> {
        if plot_idx >= self.plots.len() {
            return None;
        }
        let crop = self.plots[plot_idx].as_ref()?;
        if !crop.is_ready() {
            return None;
        }
        let gold = crop.yield_gold;
        self.plots[plot_idx] = None;
        Some(gold)
    }

    /// Start breeding an animal. Records the current Unix time.
    pub fn start_breeding(&mut self, animal_idx: usize) -> Result<(), String> {
        let animal = self.animals.get_mut(animal_idx).ok_or("无效动物索引。")?;
        if animal.breeding {
            return Err(format!("{} 已经在繁殖中了。", animal.name));
        }
        animal.breeding_start_unix = Some(now_unix());
        animal.breeding = true;
        Ok(())
    }

    /// Collect yield from a ready animal. Returns the gold earned, or `None` if not ready.
    pub fn collect_animal(&mut self, animal_idx: usize) -> Option<u32> {
        let animal = self.animals.get_mut(animal_idx)?;
        if !animal.is_ready() {
            return None;
        }
        let gold = animal.yield_gold;
        animal.breeding_start_unix = None;
        animal.breeding = false;
        Some(gold)
    }
}
