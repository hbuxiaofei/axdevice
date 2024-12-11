use core::cell::RefCell;
use axerrno::AxResult;
use axaddrspace::GuestPhysAddr;
use axdevice_base::{BaseDeviceOps, EmuDeviceType};
use memory_addr::AddrRange;

const RTC_IO_BASE: usize = 0x0070;
const NMI_DISABLE_BIT: u8 = 0x80;

const CMOS_MEM_EXTMEM: (u8, u8) = (0x30, 0x31);
const CMOS_MEM_EXTMEM2: (u8, u8) = (0x34, 0x35);

pub struct RtcDevice {
    base: GuestPhysAddr,
    cmos_data: [u8; 128],
    mem_size: u64,
    cur_index: RefCell<u8>,
}

impl RtcDevice {
    pub fn new() -> RtcDevice {
        let mut rtc = RtcDevice {
            base: GuestPhysAddr::from(RTC_IO_BASE),
            cmos_data: [0; 128],
            mem_size: 0,
            cur_index: RefCell::new(0),
        };
        rtc.set_memory(64 * 1024 * 1024);

        rtc
    }

    pub fn set_memory(&mut self, mem_size: u64) {
        self.mem_size = mem_size;

        let kb: u64 = 1024;
        let mb: u64 = 1024 * kb;

        if self.mem_size <= 64 * mb && self.mem_size > 1 * mb {
            let mem_ext = self.mem_size - 1 * mb;
            self.cmos_data[CMOS_MEM_EXTMEM.0 as usize] = (mem_ext >> 10) as u8;
            self.cmos_data[CMOS_MEM_EXTMEM.1 as usize] = (mem_ext >> 18) as u8;
        }

        if self.mem_size > 64 * mb {
            let mem_ext2 = self.mem_size - 16 * mb;
            self.cmos_data[CMOS_MEM_EXTMEM2.0 as usize] = (mem_ext2 >> 16) as u8;
            self.cmos_data[CMOS_MEM_EXTMEM2.1 as usize] = (mem_ext2 >> 24) as u8;
        }
    }

    pub fn select_index(&self, index: u8) {
        let index = index & (!NMI_DISABLE_BIT);
        debug!("Rtc select {index:#x}\n");
        *self.cur_index.borrow_mut() = index;
    }

    pub fn get_data(&self) -> AxResult<u8>  {
        let index = *self.cur_index.borrow();
        let (mem_ext0, mem_ext1) = CMOS_MEM_EXTMEM;
        let (mem2_ext0, mem2_ext1) = CMOS_MEM_EXTMEM2;
        debug!("Rtc get index: {:#x}\n", index);
        match index {
            mem_ext0 => Ok(self.cmos_data[mem_ext0 as usize]),
            mem_ext1 => Ok(self.cmos_data[mem_ext1 as usize]),
            mem2_ext0 => Ok(self.cmos_data[mem2_ext0 as usize]),
            mem2_ext1 => Ok(self.cmos_data[mem2_ext1 as usize]),
            _ => Ok(0)
        }
    }
}

impl BaseDeviceOps for RtcDevice {
    fn emu_type(&self) -> EmuDeviceType {
        EmuDeviceType::EmuDeviceTIOMMU
    }
    fn address_range(&self) -> AddrRange<GuestPhysAddr> {
        AddrRange::new(self.base.into(), (self.base + 2).into())
    }
    fn handle_read(&self, addr: GuestPhysAddr, width: usize) -> AxResult<usize> {
        debug!("Rtc read addr: {:#x} {:#x}\n", addr, self.base + 1);
        if addr == self.base + 1 {
            return self.get_data().map(|x| x.into());
        }
        Ok(0)
    }
    fn handle_write(&self, addr: GuestPhysAddr, width: usize, val: usize) {
        if addr == self.base {
            self.select_index(val as u8);
        }
    }
}
