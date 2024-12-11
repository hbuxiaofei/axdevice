use core::cell::RefCell;
use alloc::vec;
use alloc::vec::Vec;
use axerrno::AxResult;
use axaddrspace::GuestPhysAddr;
use axdevice_base::{BaseDeviceOps, EmuDeviceType};
use memory_addr::AddrRange;

const FW_CFG_IO_BASE: usize = 0x510;

const FW_CFG_FILE_SLOTS_DFLT: u16 = 0x20;
const FW_CFG_FILE_FIRST: u16 = 0x20;
const FW_CFG_MAX_ENTRY: u16 = FW_CFG_FILE_FIRST + FW_CFG_FILE_SLOTS_DFLT;
const FW_CFG_INVALID: u16 = 0xffff;

const FW_CFG_WRITE_CHANNEL: u16 = 0x4000;
const FW_CFG_ARCH_LOCAL: u16 = 0x8000;
const FW_CFG_ENTRY_MASK: u16 = !(FW_CFG_WRITE_CHANNEL | FW_CFG_ARCH_LOCAL);

#[repr(u16)]
pub enum FwCfgEntryType {
    Signature = 0x0,
    Id        = 0x1,
    E820Table = 0x8003,
}

#[derive(Clone, Default)]
struct FwCfgEntry {
    data: Vec<u8>,
}

pub struct FwCfgDevice {
    entries: Vec<FwCfgEntry>,
    base: GuestPhysAddr,
    cur_entry: RefCell<u16>,
    cur_offset: RefCell<u32>,
}

impl FwCfgDevice {
    pub fn new() -> FwCfgDevice {
        let mut cfg = FwCfgDevice {
            entries: vec![FwCfgEntry::default(); FW_CFG_MAX_ENTRY as usize],
            base: GuestPhysAddr::from(FW_CFG_IO_BASE),
            cur_entry: RefCell::new(0),
            cur_offset: RefCell::new(0),
        };
        let sig = &[b'Q', b'E', b'M', b'U'];
        cfg.add_entry(FwCfgEntryType::Signature, sig.to_vec());

        cfg
    }

    fn max_entry(&self) -> u16 {
        FW_CFG_FILE_FIRST + FW_CFG_FILE_SLOTS_DFLT
    }

    pub fn add_entry(&mut self, key: FwCfgEntryType, data: Vec<u8>)  {
        let key = (key as u16) & FW_CFG_ENTRY_MASK;
        if key < self.max_entry(){
            let entry = self.entries.get_mut(key as usize);
            if entry.is_some() {
                let entry = entry.unwrap();
                entry.data = data;
            }
        }
    }

    pub fn select_entry(&self, key: u16) {
        *self.cur_offset.borrow_mut() = 0;
        if (key & FW_CFG_ENTRY_MASK) >= self.max_entry() {
            *self.cur_entry.borrow_mut() = FW_CFG_INVALID;
        } else {
            *self.cur_entry.borrow_mut() = key;
        }
    }

    pub fn get_data(&self) -> AxResult<u8> {
        let pos = *self.cur_entry.borrow();
        let entry = self.entries.get(pos as usize);
        if entry.is_some() {
            let entry = entry.unwrap();
            let offset = *self.cur_offset.borrow();
            let r = entry.data[offset as usize];
            *self.cur_offset.borrow_mut() += 1;
            return Ok(r);
        }
        return Ok(0);
    }
}

impl BaseDeviceOps for FwCfgDevice {
    fn emu_type(&self) -> EmuDeviceType {
        EmuDeviceType::EmuDeviceTIOMMU
    }
    fn address_range(&self) -> AddrRange<GuestPhysAddr> {
        // Selector Register IOport: 0x510
        // Data Register IOport:     0x511
        AddrRange::new(self.base.into(), (self.base + 2).into())
    }
    fn handle_read(&self, addr: GuestPhysAddr, width: usize) -> AxResult<usize> {
        if addr == self.base + 1 {
            return self.get_data().map(|x| x.into());
        }
        Ok(0)

    }
    fn handle_write(&self, addr: GuestPhysAddr, width: usize, val: usize) {
        if addr == self.base {
            self.select_entry(val as u16);
        }
    }
}

