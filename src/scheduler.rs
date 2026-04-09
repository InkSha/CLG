use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct ScheduledTask {
    pub id: u64,
    pub description: String,
    pub due_at: Instant,
    pub completed: bool,
}

pub struct Scheduler {
    tasks: Arc<Mutex<Vec<ScheduledTask>>>,
    next_id: Arc<Mutex<u64>>,
}

impl Scheduler {
    pub fn new() -> Self {
        let tasks: Arc<Mutex<Vec<ScheduledTask>>> = Arc::new(Mutex::new(Vec::new()));
        let tasks_clone = Arc::clone(&tasks);

        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            let mut tasks = tasks_clone.lock().unwrap();
            let now = Instant::now();
            for task in tasks.iter_mut() {
                if !task.completed && now >= task.due_at {
                    task.completed = true;
                }
            }
        });

        Scheduler {
            tasks,
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    pub fn add_task(&self, description: String, delay_secs: u64) -> u64 {
        let mut id_guard = self.next_id.lock().unwrap();
        let id = *id_guard;
        *id_guard += 1;
        drop(id_guard);

        let task = ScheduledTask {
            id,
            description,
            due_at: Instant::now() + std::time::Duration::from_secs(delay_secs),
            completed: false,
        };

        self.tasks.lock().unwrap().push(task);
        id
    }

    pub fn is_task_completed(&self, id: u64) -> bool {
        let tasks = self.tasks.lock().unwrap();
        tasks.iter().find(|t| t.id == id).map(|t| t.completed).unwrap_or(false)
    }

    pub fn remove_task(&self, id: u64) {
        let mut tasks = self.tasks.lock().unwrap();
        tasks.retain(|t| t.id != id);
    }

    pub fn get_completed_tasks(&self) -> Vec<u64> {
        let tasks = self.tasks.lock().unwrap();
        tasks.iter().filter(|t| t.completed).map(|t| t.id).collect()
    }
}
