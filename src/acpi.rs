use core::slice;

use alloc::{borrow::Cow, string::String, vec::Vec};
use multiboot2::BootInformation;

use crate::{
    memory::{
        paging::{entry::EntryFlags, PhysAddr},
        MemoryController,
    },
    println,
};

#[repr(C)]
struct Xsdt {
    header: AcpiSDTHeader,
    entries: Vec<u64>,
}

trait AcpiEntry {
    fn sig() -> &'static str;
}

struct Madt;

impl AcpiEntry for Madt {
    fn sig() -> &'static str {
        "APIC"
    }
}

#[repr(C)]
struct Rsdt {
    header: AcpiSDTHeader,
    entries: Vec<AcpiSDTHeader>,
}

impl Rsdt {
    /// Reads the RSDT from its physical address.
    ///
    /// # Safety
    /// The caller must ensure that `addr` is aligned, readable and
    /// the RSDT is located at `addr`.
    unsafe fn new(addr: usize) -> Rsdt {
        let p_header = addr as *const AcpiSDTHeader;
        let p_entry0 = p_header.add(1) as *const u32;

        let header = core::ptr::read(p_header);
        let num_entries = (header.length - size_of::<AcpiSDTHeader>() as u32) / 4;

        let entries = slice::from_raw_parts(p_entry0, num_entries as usize);
        let entries = entries
            .iter()
            .map(|entry| AcpiSDTHeader::new(*entry))
            .collect::<Vec<_>>();

        Rsdt { header, entries }
    }

    fn checksum_is_valid(&self) -> bool {
        let ptr = self as *const _ as *const u8;
        let len = size_of::<Self>();

        // Safety: ptr and len will always be valid.
        let data = unsafe { slice::from_raw_parts(ptr, len) };
        data.iter().fold(0u8, |a, &b| a.wrapping_add(b)) == 0
    }

    fn has<T: AcpiEntry>(&self) -> bool {
        self.entries.iter().any(|x| x.signature() == T::sig())
    }

    fn fadt(&self) -> Option<AcpiSDTHeader> {
        let fadt = self
            .entries
            .iter()
            .find(|&entry| entry.signature() == "FACP")?;
        // if !fadt.checksum_is_valid() {
        // println!("len: {}, sig: {}", fadt.length, fadt.signature());
        // println!("Invalid checksum!");
        // return None;
        // }

        let fadt = fadt as *const AcpiSDTHeader;
        // Safety: ACPISDTHeader is valid
        unsafe { Some(core::ptr::read(fadt)) }
    }
}

#[repr(C)]
struct AcpiSDTHeader {
    /// Used to determine what table we look at.
    signature: [u8; 4],
    /// Total size of the table, including the header.
    length: u32,
    revision: u8,
    /// Sum of all bytes must be equal to 0 (mod 0x100).
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}

impl AcpiSDTHeader {
    unsafe fn new(addr: u32) -> AcpiSDTHeader {
        core::ptr::read(addr as *const AcpiSDTHeader)
    }

    fn signature(&self) -> Cow<str> {
        String::from_utf8_lossy(&self.signature)
    }
}

/// Attemps to setup ACPI.
pub fn try_init(mbi: &BootInformation, memory_controller: &mut MemoryController) {
    if let Some(rsdp) = mbi.rsdp_v2_tag() {
        // RSDP v2
        if !rsdp.checksum_is_valid() {
            println!("[!] RSDP checksum was not valid.");
            return;
        }

        let xsdt: PhysAddr = rsdp.xsdt_address();
    } else if let Some(rsdp) = mbi.rsdp_v1_tag() {
        // RSDP v1
        if !rsdp.checksum_is_valid() {
            println!("[!] RSDP checksum was not valid.");
            return;
        }

        println!("found RSDP v1 with RSDT at {:#x}", rsdp.rsdt_address());

        memory_controller.id_map(rsdp.rsdt_address(), EntryFlags::PRESENT);
        // Safety: rsdp is valid
        let rsdt = unsafe { Rsdt::new(rsdp.rsdt_address()) };

        for entry in rsdt.entries {
            println!("Found {}.", entry.signature());
        }
    } else {
        println!("No ACPI found");
    }
}
