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

pub trait AxisBehaviourTrait {
    fn get(&mut self, data: u8) -> u8;
    fn set(&mut self, data: u8);
}

#[derive(Clone, Serialize, Deserialize, Debug, Copy)]
pub enum AxisBehaviour {
    Absolute(AbsoluteAxis),
    CustomRelative(CustomRelativeAxis),
    TwosComplimentRelative(TwosComplimentRelativeAxis),
    SignMagnitudeRelative(SignMagnitudeRelativeAxis),
    BinaryOffsetRelative(BinaryOffsetRelativeAxis),
}

#[derive(Clone, Serialize, Deserialize, Debug, Copy)]
pub struct AbsoluteAxis {
    min_in: u8,
    max_in: u8,
    min_out: u8,
    max_out: u8,
}

#[derive(Clone, Serialize, Deserialize, Debug, Copy)]
pub struct CustomRelativeAxis {
    curr_val: u8,
    threshold: u8,
    invert: bool,
    step: u8,
}

#[derive(Clone, Serialize, Deserialize, Debug, Copy)]
pub struct TwosComplimentRelativeAxis {
    curr_val: u8,
    invert: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug, Copy)]
pub struct SignMagnitudeRelativeAxis {
    curr_val: u8,
    invert: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug, Copy)]
pub struct BinaryOffsetRelativeAxis {
    curr_val: u8,
    invert: bool,
}

impl AxisBehaviourTrait for AxisBehaviour {
    fn get(&mut self, data: u8) -> u8 {
        match self {
            AxisBehaviour::Absolute(a) => a.get(data),
            AxisBehaviour::CustomRelative(a) => a.get(data),
            AxisBehaviour::TwosComplimentRelative(a) => a.get(data),
            AxisBehaviour::SignMagnitudeRelative(a) => a.get(data),
            AxisBehaviour::BinaryOffsetRelative(a) => a.get(data),
        }
    }
    fn set(&mut self, data: u8) {
        match self {
            AxisBehaviour::Absolute(a) => a.set(data),
            AxisBehaviour::CustomRelative(a) => a.set(data),
            AxisBehaviour::TwosComplimentRelative(a) => a.set(data),
            AxisBehaviour::SignMagnitudeRelative(a) => a.set(data),
            AxisBehaviour::BinaryOffsetRelative(a) => a.set(data),
        }
    }
}

impl AxisBehaviourTrait for AbsoluteAxis {
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
            max_out,
        }
    }
}

impl AxisBehaviourTrait for CustomRelativeAxis {
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

impl CustomRelativeAxis {
    pub fn new(threshold: u8, invert: Option<bool>, step: Option<u8>) -> CustomRelativeAxis {
        Self {
            curr_val: 0,
            threshold,
            invert: invert.unwrap_or(false),
            step: step.unwrap_or(1),
        }
    }
}

impl AxisBehaviourTrait for TwosComplimentRelativeAxis {
    fn get(&mut self, data: u8) -> u8 {
        let mut offset;
        if data & 0b0100_0000 != 0 {
            offset = -(((data & 0b0111_1111) + 1) as i8) // 2's compliment negate 7 bit to get a valid u8, then negate it to get the proper negation.
        } else {
            offset = data as i8;
        }
        if self.invert {
            offset = -offset;
        }
        self.curr_val = self.curr_val.saturating_add_signed(offset);
        if self.curr_val > 100 {
            self.curr_val = 100;
        }
        self.curr_val
    }

    fn set(&mut self, data: u8) {
        self.curr_val = data;
    }
}

impl TwosComplimentRelativeAxis {
    pub fn new(invert: Option<bool>) -> TwosComplimentRelativeAxis {
        Self {
            curr_val: 0,
            invert: invert.unwrap_or(false),
        }
    }
}

impl AxisBehaviourTrait for SignMagnitudeRelativeAxis {
    fn get(&mut self, data: u8) -> u8 {
        let mut offset;
        if data & 0b0100_0000 != 0 {
            let data = data & 0b0011_1111;
            offset = data as i8;
        } else {
            offset = data as i8;
        };
        if self.invert {
            offset = -offset;
        }
        self.curr_val = self.curr_val.saturating_add_signed(offset);
        if self.curr_val > 100 {
            self.curr_val = 100;
        }
        self.curr_val
    }

    fn set(&mut self, data: u8) {
        self.curr_val = data;
    }
}

impl SignMagnitudeRelativeAxis {
    pub fn new(invert: Option<bool>) -> SignMagnitudeRelativeAxis {
        Self {
            curr_val: 0,
            invert: invert.unwrap_or(false),
        }
    }
}

impl AxisBehaviourTrait for BinaryOffsetRelativeAxis {
    fn get(&mut self, data: u8) -> u8 {
        let mut offset = (data as i16 - 64) as i8;
        if self.invert {
            offset = -offset;
        }
        self.curr_val = self.curr_val.saturating_add_signed(offset);
        if self.curr_val > 100 {
            self.curr_val = 100;
        }
        self.curr_val
    }

    fn set(&mut self, data: u8) {
        self.curr_val = data;
    }
}

impl BinaryOffsetRelativeAxis {
    pub fn new(invert: Option<bool>) -> BinaryOffsetRelativeAxis {
        Self {
            curr_val: 0,
            invert: invert.unwrap_or(false),
        }
    }
}
