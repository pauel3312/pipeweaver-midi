use serde::{Deserialize, Serialize};

pub trait BooleanBehaviourTrait {
    fn get(&mut self, data: u8) -> bool;
    fn set(&mut self, data: bool);
}

#[derive(Clone, Serialize, Deserialize, Debug, Copy)]
pub enum BooleanBehaviour {
    Toggle(ToggleBtn),
    Push(PushBtn),
}

#[derive(Clone, Serialize, Deserialize, Debug, Copy)]
pub struct ToggleBtn {
    current_state: bool,
    previous_val: bool,
    threshold: u8,
    falling_edge: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug, Copy)]
pub struct PushBtn {
    threshold: u8,
    invert: bool,
}

impl BooleanBehaviourTrait for BooleanBehaviour {
    fn get(&mut self, data: u8) -> bool {
        match self {
            BooleanBehaviour::Toggle(toggle) => toggle.get(data),
            BooleanBehaviour::Push(push) => push.get(data),
        }
    }

    fn set(&mut self, data: bool) {
        match self {
            BooleanBehaviour::Toggle(toggle) => toggle.set(data),
            BooleanBehaviour::Push(push) => push.set(data),
        }
    }
}

impl BooleanBehaviourTrait for ToggleBtn {
    fn get(&mut self, data: u8) -> bool {
        let mut check = data > self.threshold;
        if self.falling_edge {
            check = !check;
        }
        if self.previous_val == check {
            self.current_state
        } else {
            // Changed
            if check {
                self.current_state = !self.current_state;
            }
            self.previous_val = check;
            self.current_state
        }
    }
    fn set(&mut self, data: bool) {
        self.current_state = data;
    }
}

impl ToggleBtn {
    pub fn new(threshold: u8, falling_edge: Option<bool>) -> Self {
        Self {
            current_state: false,
            previous_val: false,
            threshold,
            falling_edge: falling_edge.unwrap_or(false),
        }
    }
}
impl BooleanBehaviourTrait for PushBtn {
    fn get(&mut self, data: u8) -> bool {
        if self.invert {
            data <= self.threshold
        } else {
            data >= self.threshold
        }
    }
    fn set(&mut self, _: bool) {}
}

impl PushBtn {
    pub fn new(threshold: u8, falling_edge: Option<bool>) -> Self {
        Self {
            threshold,
            invert: falling_edge.unwrap_or(false),
        }
    }
}
