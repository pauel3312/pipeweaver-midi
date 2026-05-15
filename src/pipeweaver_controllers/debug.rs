use crate::pipeweaver_controllers::core::{AxisProvider, BooleanProvider, CallbackProvider};

#[allow(unused)] // Useful for debugging.
#[derive(Debug)]
pub struct PrinterController {
    pub name: String,
}

impl CallbackProvider for PrinterController {
    fn callback(&self, data: u8) {
        println!("Callback for {} with data {}", self.name, data);
    }
}

impl AxisProvider for PrinterController {
    fn set(&self, data: u8) {
        println!("Axis set for {} with data {}", self.name, data);
    }
}

impl BooleanProvider for PrinterController {
    fn set(&self, data: bool) {
        println!("Bool set for {} with data {}", self.name, data);
    }
}
