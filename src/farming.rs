use serde::{Deserialize, Serialize};
use crate::scheduler::Scheduler;

#[derive(Serialize, Deserialize, Clone)]
pub struct Crop {
    pub name: String,
    pub grow_time_secs: u64,
    pub yield_gold: u32,
    pub task_id: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Animal {
    pub name: String,
    pub breed_time_secs: u64,
    pub yield_gold: u32,
    pub task_id: Option<u64>,
    pub breeding: bool,
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
                    task_id: None,
                    breeding: false,
                },
                Animal {
                    name: "牛".to_string(),
                    breed_time_secs: 300,
                    yield_gold: 50,
                    task_id: None,
                    breeding: false,
                },
                Animal {
                    name: "羊".to_string(),
                    breed_time_secs: 180,
                    yield_gold: 25,
                    task_id: None,
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

    pub fn plant(
        &mut self,
        plot_idx: usize,
        crop_type_idx: usize,
        scheduler: &Scheduler,
    ) -> Result<u64, String> {
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

        let task_id = scheduler.add_task(
            format!("Grow {}", name),
            grow_time,
        );

        self.plots[plot_idx] = Some(Crop {
            name,
            grow_time_secs: grow_time,
            yield_gold,
            task_id: Some(task_id),
        });

        Ok(task_id)
    }

    pub fn harvest(&mut self, plot_idx: usize, scheduler: &Scheduler) -> Option<u32> {
        if plot_idx >= self.plots.len() {
            return None;
        }
        let crop = self.plots[plot_idx].as_ref()?;
        let task_id = crop.task_id?;

        if scheduler.is_task_completed(task_id) {
            let gold = crop.yield_gold;
            scheduler.remove_task(task_id);
            self.plots[plot_idx] = None;
            Some(gold)
        } else {
            None
        }
    }

    pub fn start_breeding(
        &mut self,
        animal_idx: usize,
        scheduler: &Scheduler,
    ) -> Result<u64, String> {
        let animal = self.animals.get_mut(animal_idx).ok_or("无效动物索引。")?;
        if animal.breeding {
            return Err(format!("{} 已经在繁殖中了。", animal.name));
        }

        let task_id = scheduler.add_task(
            format!("Breed {}", animal.name),
            animal.breed_time_secs,
        );
        animal.task_id = Some(task_id);
        animal.breeding = true;
        Ok(task_id)
    }

    pub fn collect_animal(&mut self, animal_idx: usize, scheduler: &Scheduler) -> Option<u32> {
        let animal = self.animals.get_mut(animal_idx)?;
        if !animal.breeding {
            return None;
        }
        let task_id = animal.task_id?;

        if scheduler.is_task_completed(task_id) {
            let gold = animal.yield_gold;
            scheduler.remove_task(task_id);
            animal.task_id = None;
            animal.breeding = false;
            Some(gold)
        } else {
            None
        }
    }
}
