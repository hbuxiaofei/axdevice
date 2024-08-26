#![no_std]

extern crate alloc;
#[macro_use]
extern crate log;

pub use emu_device::EmulatedDeviceConfig;
mod config;
pub use config::AxVmDeviceConfig;

mod device;
pub use device::AxVmDevices;
