use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Player {
    pub name: String,
    pub hp: i32,
    pub max_hp: i32,
    pub attack: i32,
    pub defense: i32,
    pub level: u32,
    pub exp: u32,
    pub exp_to_next: u32,
    pub gold: u32,
}

impl Player {
    pub fn new(name: String) -> Self {
        Player {
            name,
            hp: 100,
            max_hp: 100,
            attack: 10,
            defense: 5,
            level: 1,
            exp: 0,
            exp_to_next: 100,
            gold: 50,
        }
    }

    pub fn take_damage(&mut self, amt: i32) {
        self.hp = (self.hp - amt).max(0);
    }

    pub fn heal(&mut self, amt: i32) {
        self.hp = (self.hp + amt).min(self.max_hp);
    }

    /// Returns true if player leveled up
    pub fn gain_exp(&mut self, amt: u32) -> bool {
        self.exp += amt;
        if self.exp >= self.exp_to_next {
            self.level_up();
            true
        } else {
            false
        }
    }

    fn level_up(&mut self) {
        self.exp -= self.exp_to_next;
        self.level += 1;
        self.exp_to_next = self.level * 100;
        self.max_hp += 20;
        self.hp = self.max_hp;
        self.attack += 3;
        self.defense += 2;
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }
}
