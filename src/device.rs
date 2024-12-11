use crate::AxVmDeviceConfig;
use crate::FwCfgDevice;
use crate::RtcDevice;
use crate::PciDevice;

use alloc::sync::Arc;
use alloc::vec::Vec;

use axaddrspace::GuestPhysAddr;
use axdevice_base::EmulatedDeviceConfig;
use axdevice_base::{BaseDeviceOps, EmuDeviceType};
use axerrno::AxResult;

/// represent A vm own devices
pub struct AxVmDevices {
    /// emu devices
    emu_devices: Vec<Arc<dyn BaseDeviceOps>>,
    // TODO passthrough devices or other type devices ...
}

/// The implemention for AxVmDevices
impl AxVmDevices {
    /// According AxVmDeviceConfig to init the AxVmDevices
    pub fn new(config: AxVmDeviceConfig) -> Self {
        let mut this = Self {
            emu_devices: Vec::new(),
        };

        Self::init(&mut this, &config.emu_configs);
        this
    }

    /// According the emu_configs to init every  specific device
    fn init(this: &mut Self, emu_configs: &Vec<EmulatedDeviceConfig>) {
        this.emu_devices.push(Arc::new(FwCfgDevice::new()));
        this.emu_devices.push(Arc::new(RtcDevice::new()));
        this.emu_devices.push(Arc::new(PciDevice::new()));
        /*
        for config in emu_configs {
            let dev = match EmuDeviceType::from_usize(config.emu_type) {
                // todo call specific initialization function of devcise
                EmuDeviceType::EmuDeviceTConsole => ,
                EmuDeviceType::EmuDeviceTGicdV2 => ,
                EmuDeviceType::EmuDeviceTGPPT => ,
                EmuDeviceType::EmuDeviceTVirtioBlk => ,
                EmuDeviceType::EmuDeviceTVirtioNet => ,
                EmuDeviceType::EmuDeviceTVirtioConsole => ,
                EmuDeviceType::EmuDeviceTIOMMU => ,
                EmuDeviceType::EmuDeviceTICCSRE => ,
                EmuDeviceType::EmuDeviceTSGIR => ,
                EmuDeviceType::EmuDeviceTGICR => ,
                EmuDeviceType::EmuDeviceTMeta => ,
                _ => panic!("emu type: {} is still not supported", config.emu_type),
            };
            if let Ok(emu_dev) = dev {
                this.emu_devices.push(emu_dev)
            }
        }
        */
    }

    /// Find specific device by ipa
    pub fn find_dev(&self, ipa: GuestPhysAddr) -> Option<Arc<dyn BaseDeviceOps>> {
        self.emu_devices
            .iter()
            .find(|&dev| dev.address_range().contains(ipa))
            .cloned()
    }

    /// Handle the io read by port
    pub fn handle_port_read(&self, port: u16, width: usize) ->  AxResult<usize> {
        let port = GuestPhysAddr::from(port as usize);
        if let Some(emu_dev) = self.find_dev(port) {
            debug!(
                "emu: {:?} handler read port:{:#x} width:{:}",
                emu_dev.address_range(),
                port, width
            );
            return emu_dev.handle_read(port, width);
        }
        panic!(
            "emu_handler: no emul handler read for data abort port {:#x}",
            port
        );
    }

    /// Handle the io write by port
    pub fn handle_port_write(&self, port: u16, width: usize, val: usize) {
        let port = GuestPhysAddr::from(port as usize);
        if let Some(emu_dev) = self.find_dev(port) {
            debug!(
                "emu: {:?} handler write port:{:#x} width:{:} val:{:#x}",
                emu_dev.address_range(),
                port, width, val
            );
            emu_dev.handle_write(port, width, val);
            return;
        }
        panic!(
            "emu_handler: no emul handler write for data abort port {:#x}",
            port
        );
    }

    /// Handle the MMIO read by GuestPhysAddr and data width, return the value of the guest want to read
    pub fn handle_mmio_read(&self, addr: GuestPhysAddr, width: usize) -> AxResult<usize> {
        if let Some(emu_dev) = self.find_dev(addr) {
            info!(
                "emu: {:?} handler read ipa {:#x}",
                emu_dev.address_range(),
                addr
            );
            return emu_dev.handle_read(addr, width);
        }
        panic!("emu_handle: no emul handler for data abort ipa {:#x}", addr);
    }

    /// Handle the MMIO write by GuestPhysAddr, data width and the value need to write, call specific device to write the value
    pub fn handle_mmio_write(&self, addr: GuestPhysAddr, width: usize, val: usize) {
        if let Some(emu_dev) = self.find_dev(addr) {
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
