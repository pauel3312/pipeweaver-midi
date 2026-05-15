use crate::behaviours::axis_behaviours::{
    AbsoluteAxis, AxisBehaviour, BinaryOffsetRelativeAxis, CustomRelativeAxis, SignMagnitudeRelativeAxis,
    TwosComplimentRelativeAxis,
};
use crate::behaviours::button_behaviours::{BooleanBehaviour, PushBtn, ToggleBtn};
use crate::ui::behaviour_selectors::AxisBehaviourState::*;

#[derive(Debug, Copy, Clone)]
pub(super) enum BoolBehaviourState {
    Toggle { threshold: u8, invert: bool },
    Push { threshold: u8, invert: bool },
}

#[derive(Debug, Copy, Clone)]
pub(super) enum AxisBehaviourState {
    BinaryOffsetRelative {},
    MidiAbsolute {},
    TwosComplimentRelative {},
    SignMagnitudeRelative {},
    CustomRelative { threshold: u8, invert: bool },
    CustomAbsolute { min_in: u8, max_in: u8 },
}

impl BoolBehaviourState {
    pub(super) fn make_behaviour(self) -> BooleanBehaviour {
        match self {
            BoolBehaviourState::Toggle { threshold, invert } => {
                BooleanBehaviour::Toggle(ToggleBtn::new(threshold, Some(invert)))
            }
            BoolBehaviourState::Push { threshold, invert } => {
                BooleanBehaviour::Push(PushBtn::new(threshold, Some(invert)))
            }
        }
    }
}

impl AxisBehaviourState {
    pub(super) fn make_behaviour(self) -> AxisBehaviour {
        match self {
            BinaryOffsetRelative {} => AxisBehaviour::BinaryOffsetRelative(BinaryOffsetRelativeAxis::new(None)),
            MidiAbsolute {} => AxisBehaviour::Absolute(AbsoluteAxis::new(0, 127, 0, 100)),
            TwosComplimentRelative {} => AxisBehaviour::TwosComplimentRelative(TwosComplimentRelativeAxis::new(None)),
            SignMagnitudeRelative {} => AxisBehaviour::SignMagnitudeRelative(SignMagnitudeRelativeAxis::new(None)),
            CustomRelative { threshold, invert } => {
                AxisBehaviour::CustomRelative(CustomRelativeAxis::new(threshold, Some(invert), None))
            }
            CustomAbsolute { min_in, max_in } => AxisBehaviour::Absolute(AbsoluteAxis::new(min_in, max_in, 0, 100)),
        }
    }
}
