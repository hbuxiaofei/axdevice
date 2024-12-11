use core::cell::RefCell;
use axerrno::AxResult;
use axaddrspace::GuestPhysAddr;
use axdevice_base::{BaseDeviceOps, EmuDeviceType};
use memory_addr::AddrRange;

const PCI_IO_BASE: usize = 0x0cf8;

pub struct PciDevice {
    base: GuestPhysAddr,
}

impl PciDevice {
    pub fn new() -> PciDevice {
        PciDevice {
            base: GuestPhysAddr::from(PCI_IO_BASE),
        }
    }
}

impl BaseDeviceOps for PciDevice {
    fn emu_type(&self) -> EmuDeviceType {
        EmuDeviceType::EmuDeviceTIOMMU
    }
    fn address_range(&self) -> AddrRange<GuestPhysAddr> {
        AddrRange::new(self.base.into(), (self.base + 8).into())
    }
    fn handle_read(&self, addr: GuestPhysAddr, width: usize) -> AxResult<usize> {
        Ok(0)
    }
    fn handle_write(&self, addr: GuestPhysAddr, width: usize, val: usize) {
    }
}
