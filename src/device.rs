use alloc::sync::Arc;
use alloc::vec::Vec;

use emu_device::EmuDev;
use emu_device::EmulatedDeviceConfig;

use crate::AxVmDeviceConfig;
use axerrno::AxResult;

// represent A vm own devices
pub struct AxVmDevices {
    // emu devices
    emu_devices: Vec<Arc<dyn EmuDev>>,
    // TODO passthrough devices or other type devices ...
}

impl AxVmDevices {
    fn init_emu_dev(this: &mut Self, emu_configs: &Vec<EmulatedDeviceConfig>) {
        for config in emu_configs {
            let dev = config.to_emu_dev();
            if let Ok(emu_dev) = dev {
                this.emu_devices.push(emu_dev)
            }
        }
    }

    pub fn new(config: AxVmDeviceConfig) -> Self {
        let mut this = Self {
            emu_devices: Vec::new(),
        };

        Self::init_emu_dev(&mut this, &config.emu_configs);
        this
    }

    pub fn find_emu_dev(&self, ipa: usize) -> Option<Arc<dyn EmuDev>> {
        self.emu_devices
            .iter()
            .find(|&dev| dev.address_range().contains(&ipa))
            .cloned()
    }

    pub fn handle_emu_read(&self, addr: usize, width: usize) -> AxResult<usize> {
        if let Some(emu_dev) = self.find_emu_dev(addr) {
            info!(
                "emu: {:?} handler read ipa {:#x}",
                emu_dev.address_range(),
                addr
            );
            return emu_dev.handle_read(addr, width);
        }

        panic!("emu_handle: no emul handler for data abort ipa {:#x}", addr);
    }

    pub fn handle_emu_write(&self, addr: usize, width: usize, val: usize) {
        if let Some(emu_dev) = self.find_emu_dev(addr) {
            info!(
                "emu: {:?} handler write ipa {:#x}",
                emu_dev.address_range(),
                addr
            );
            emu_dev.handle_write(addr, width, val);
            return;
        }
        panic!(
            "emu_handler: no emul handler for data abort ipa {:#x}",
            addr
        );
    }
}
