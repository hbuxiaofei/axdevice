use alloc::vec::Vec;
use emu_device::EmulatedDeviceConfig;

pub struct AxVmDeviceConfig {
    pub emu_configs: Vec<EmulatedDeviceConfig>,
}

impl AxVmDeviceConfig {
    pub fn new(emu_configs: Vec<EmulatedDeviceConfig>) -> Self {
        Self { emu_configs }
    }
}
