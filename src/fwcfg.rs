use core::cell::RefCell;
use alloc::vec;
use alloc::vec::Vec;
use alloc::string::ToString;
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
    FileDir   = 0x19,
    E820Table = 0x8003,
}

#[derive(Clone, Default)]
struct FwCfgEntry {
    data: Vec<u8>,
    allow_write: bool,
}

impl FwCfgEntry {
    fn new(
        data: Vec<u8>,
        allow_write: bool,
    ) -> Self {
        FwCfgEntry {
            data,
            allow_write,
        }
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
struct FwCfgFile {
    size: u32,
    select: u16,
    reserved: u16,
    name: [u8; 56],
}

impl FwCfgFile {
    fn new(size: u32, select: u16, name: &str) -> Self {
        let len = core::cmp::min(56, name.len());
        let mut bytes = [0; 56];
        bytes[..len].copy_from_slice(&name.as_bytes()[..len]);

        FwCfgFile {
            size,
            select,
            reserved: 0,
            name: bytes,
        }
    }

    fn as_be_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![0; 64_usize];

        let mut curr_offset = 0_usize;

        bytes[curr_offset..curr_offset + 4].copy_from_slice(&self.size.to_be_bytes());
        curr_offset += 4;

        bytes[curr_offset..curr_offset + 2].copy_from_slice(&self.select.to_be_bytes());
        curr_offset += 2;

        bytes[curr_offset..curr_offset + 2].copy_from_slice(&self.reserved.to_be_bytes());
        curr_offset += 2;

        bytes[curr_offset..].copy_from_slice(&self.name);

        bytes
    }
}

pub struct FwCfgDevice {
    entries: Vec<FwCfgEntry>,
    base: GuestPhysAddr,
    cur_entry: RefCell<u16>,
    cur_offset: RefCell<u32>,
    files: Vec<FwCfgFile>,
}

impl FwCfgDevice {
    pub fn new() -> FwCfgDevice {
        let mut cfg = FwCfgDevice {
            entries: vec![FwCfgEntry::default(); FW_CFG_MAX_ENTRY as usize],
            base: GuestPhysAddr::from(FW_CFG_IO_BASE),
            cur_entry: RefCell::new(0),
            cur_offset: RefCell::new(0),
            files: Vec::new(),
        };
        // let sig = &[b'R', b'V', b'M', b'\0'];
        let sig = &[b'Q', b'E', b'M', b'U'];
        cfg.add_entry(FwCfgEntryType::Signature, sig.to_vec());

        let data = &[b'H', b'E', b'L', b'L', b'O'];
        cfg.add_file("genroms/multiboot.bin", data.to_vec(), false);

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

    fn update_entry_data(&mut self, key: u16, mut data: Vec<u8>) {
        let entry = self.entries.get_mut(key as usize);

        if let Some(e) = entry {
            e.data.clear();
            e.data.append(&mut data);
        }
    }

    fn add_file(&mut self, filename: &str, data: Vec<u8>, allow_write: bool) -> AxResult<()> {
        let file_name_bytes = filename.to_string().as_bytes().to_vec();
        let mut index = self.files.len();
        for (i, file_entry) in self.files.iter().enumerate() {
            if file_name_bytes < file_entry.name.to_vec() {
                index = i;
                break;
            }
        }

        let file = FwCfgFile::new(
            data.len() as u32,
            FW_CFG_FILE_FIRST + index as u16,
            filename,
            );
        self.files.insert(index, file);
        self.files.iter_mut().skip(index + 1).for_each(|f| {
            f.select += 1;
        });

        let mut bytes = Vec::new();
        let file_length_be = self.files.len() as u32;
        bytes.append(&mut file_length_be.to_be_bytes().to_vec());
        for value in self.files.iter() {
            bytes.append(&mut value.as_be_bytes());
        }
        self.update_entry_data(FwCfgEntryType::FileDir as u16, bytes);

        self.entries.insert(
            FW_CFG_FILE_FIRST as usize + index,
            FwCfgEntry::new(data, allow_write),
            );
        Ok(())
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

