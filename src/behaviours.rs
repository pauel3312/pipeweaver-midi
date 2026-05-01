
pub trait BooleanProvider {
    fn get(&mut self, data: u8) -> bool;
    fn set(&mut self, data: bool);
}

pub struct ToggleBtn {
    current_state: bool,
    previous_val: bool,
    threshold: u8,
    falling_edge: bool,
}

pub enum PushBtn {
    Val { v_up: u8, v_down: u8 },
    Threshold { threshold: u8, invert: bool },
}

impl BooleanProvider for ToggleBtn {
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
    pub fn new(threshold: u8, falling_edge: Option<bool>) -> ToggleBtn {
        ToggleBtn {
            current_state: false,
            previous_val: false,
            threshold,
            falling_edge: falling_edge.unwrap_or(false),
        }
    }
}
impl BooleanProvider for PushBtn {
    fn get(&mut self, data: u8) -> bool {
        match *self {
            PushBtn::Val { v_up, v_down } => {
                if data == v_up {
                    true
                } else if data == v_down {
                    false
                } else {
                    false
                }
            }
            PushBtn::Threshold { threshold, invert } => {
                if invert {
                    data < threshold
                } else {
                    data > threshold
                }
            }
        }
    }
    fn set(&mut self, _: bool) {}
}

pub trait AxisProvider {
    fn get(&mut self, data: u8) -> u8;
    fn set(&mut self, data: u8);
}

pub struct AbsoluteAxis {
    min_in: u8,
    max_in: u8,
    min_out: u8,
    max_out: u8,
}

pub struct RelativeAxis {
    curr_val: u8,
    threshold: u8,
    invert: bool,
    step: u8,
}

impl AxisProvider for AbsoluteAxis {
    fn get(&mut self, data: u8) -> u8 {
        ((data - self.min_in) as f32 / (self.max_in - self.min_in) as f32
            * (self.max_out - self.min_out) as f32) as u8
            + self.min_out
    }
    fn set(&mut self, _: u8) {}
}

impl AbsoluteAxis {
    pub fn new(min_in: u8, max_in: u8, min_out: u8, max_out: u8) -> AbsoluteAxis {
        AbsoluteAxis {
            min_in,
            max_in,
            min_out,
            max_out
        }
    }
}

impl AxisProvider for RelativeAxis {
    fn get(&mut self, data: u8) -> u8 {
        let mut check = data > self.threshold;
        if self.invert {
            check = !check;
        }
        if check {
            self.curr_val += self.step;
        } else {
            self.curr_val -= self.step;
        }
        self.curr_val
    }
    fn set(&mut self, data: u8) {
        self.curr_val = data;
    }
}

impl RelativeAxis {
    pub fn new(threshold: u8, invert: Option<bool>, step: Option<u8>) -> RelativeAxis {
        RelativeAxis {
            curr_val: 0,
            threshold,
            invert: invert.unwrap_or(false),
            step: step.unwrap_or(1),
        }
    }
}
