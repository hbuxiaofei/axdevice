use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::vec;
use core::cell::RefCell;
use axerrno::AxResult;
use axaddrspace::GuestPhysAddr;
use axdevice_base::{BaseDeviceOps, EmuDeviceType};
use memory_addr::AddrRange;

const PCI_IO_BASE: usize = 0x0cf8;

#[derive(Debug, Clone, Default)]
struct PciDevice {
    vendor_id: u16,
    device_id: u16,
    class_code: u8,
    subclass: u8,
    prog_if: u8,
    revision_id: u8,
    bar: Vec<u32>,  // Base Address Registers (BARs)
}

struct PciBusInner {
    devices: BTreeMap<u8, BTreeMap<u8, PciDevice>>,  // bus -> device -> function -> PciDevice
}

impl PciBusInner {
    pub fn new() -> PciBusInner {
        PciBusInner {
            devices: BTreeMap::new(),
        }
    }
}

pub struct PciBus {
    base: GuestPhysAddr,
    config_address: RefCell<u32>,
    inner: RefCell<PciBusInner>,
}

impl PciBus {
    pub fn new() -> PciBus {
        let mut bus = PciBus {
            base: GuestPhysAddr::from(PCI_IO_BASE),
            config_address: RefCell::new(0),
            inner: RefCell::new(PciBusInner::new()),
        };

        let device = PciDevice {
            vendor_id: 0x8086,
            device_id: 0x1234,
            class_code: 0x02,
            subclass: 0x00,
            prog_if: 0x00,
            revision_id: 0x01,
            bar: vec![0x00000000, 0x00001000],
        };
        bus.add_device(0, 0, 0, device);

        let device2 = PciDevice {
            vendor_id: 0x8086,
            device_id: 0x1234,
            class_code: 0x03,
            subclass: 0x00,
            prog_if: 0x00,
            revision_id: 0x01,
            bar: vec![0x00000000, 0x00001000],
        };
        bus.add_device(0, 1, 0, device2);

        bus
    }

    fn add_device(&mut self, bus: u8, device: u8, function: u8, device_info: PciDevice) {
        let mut inner = self.inner.borrow_mut();
        let mut devices = &mut inner.devices;

        let device_map = devices.entry(bus).or_insert_with(BTreeMap::new);
        device_map.insert(device, device_info);
    }

    fn set_config_address(&self, address: u32) {
        let bus = ((address >> 16) & 0xFF) as u8;
        let device = ((address >> 11) & 0x1F) as u8;
        let function = ((address >> 8) & 0x7) as u8;
        let offset = (address & 0xFF) as u8;

        *self.config_address.borrow_mut() = address;
    }

    fn get_config_address(&self) -> u32 {
        *self.config_address.borrow()
    }

    fn read_config(&self) -> u32 {
        let config_address = self.get_config_address();
        let bus = ((config_address >> 16) & 0xFF) as u8;
        let device = ((config_address >> 11) & 0x1F) as u8;
        let function = ((config_address >> 8) & 0x7) as u8;
        let offset = (config_address & 0xFF) as u8;

        let inner = self.inner.borrow();
        let devices = &inner.devices;

        if let Some(device_map) = devices.get(&bus) {
            if let Some(device) = device_map.get(&device) {
                let bar_offset = offset as usize;
                match bar_offset {
                    0..=0x3F => {
                        // Standard PCI device header information (Vendor ID, Device ID, etc.)
                        match offset {
                            0x00 => (device.vendor_id as u32) | ((device.device_id as u32) << 16),
                            0x08 => (device.class_code as u32) | ((device.subclass as u32) << 8) | ((device.prog_if as u32) << 16),
                            0x0C => device.revision_id as u32,
                            0x10..=0x1F => {
                                if (offset as usize) >= 0x10 && (offset as usize) < device.bar.len() * 4 {
                                    // Handle Base Address Registers (BARs)
                                    let bar_index = (offset as usize - 0x10) / 4;
                                    device.bar.get(bar_index).copied().unwrap_or(0)
                                } else {
                                    0
                                }
                            }
                            _ => 0,
                        }
                    }
                    _ => 0,
                }
            } else {
                0
            }
        } else {
            0
        }
    }

    fn write_config(&self, value: u32) {
        let config_address = self.get_config_address();
        let bus = ((config_address >> 16) & 0xFF) as u8;
        let device = ((config_address >> 11) & 0x1F) as u8;
        let function = ((config_address >> 8) & 0x7) as u8;
        let offset = (config_address & 0xFF) as u8;

        let mut inner = self.inner.borrow_mut();
        let mut devices = &mut inner.devices;

        if let Some(device_map) = devices.get_mut(&bus) {
            if let Some(device) = device_map.get_mut(&device) {
                let bar_offset = offset as usize;
                match bar_offset {
                    0..=0x3F => {
                        // Handle writing to vendor ID, device ID, etc.
                        match offset {
                            0x00 => {
                                device.vendor_id = (value & 0xFFFF) as u16;
                                device.device_id = (value >> 16) as u16;
                            }
                            0x08 => {
                                device.class_code = (value & 0xFF) as u8;
                                device.subclass = ((value >> 8) & 0xFF) as u8;
                                device.prog_if = ((value >> 16) & 0xFF) as u8;
                            }
                            0x0C => {
                                device.revision_id = (value & 0xFF) as u8;
                            }
                            0x10..=0x1F => {
                                if bar_offset >= 0x10 && bar_offset < device.bar.len() * 4 {
                                    let bar_index = (bar_offset - 0x10) / 4;
                                    if bar_index < device.bar.len() {
                                        device.bar[bar_index] = value;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

impl BaseDeviceOps for PciBus {
    fn emu_type(&self) -> EmuDeviceType {
        EmuDeviceType::EmuDeviceTIOMMU
    }
    fn address_range(&self) -> AddrRange<GuestPhysAddr> {
        AddrRange::new(self.base.into(), (self.base + 8).into())
    }
    fn handle_read(&self, addr: GuestPhysAddr, width: usize) -> AxResult<usize> {
        let port: usize = addr.into();

        let r = match port {
            0xCF8 => {
                self.get_config_address()
            }
            0xCFC => {
                self.read_config()
            }
            _ => 0,
        };

        Ok(r as usize)
    }
    fn handle_write(&self, addr: GuestPhysAddr, width: usize, val: usize) {
        let port: usize = addr.into();
         match port {
            0xCF8 => {
                self.set_config_address(val as u32);
            }
            0xCFC => {
                self.write_config(val as u32);
            }
            _ => {}
        }
    }
}
