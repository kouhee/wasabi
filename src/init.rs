extern crate alloc;

use crate::allocator::ALLOCATOR;
use crate::uefi::exit_from_efi_boot_services;
use crate::uefi::EfiHandle;
use crate::uefi::EfiMemoryType::*;
use crate::uefi::EfiSystemTable;
use crate::uefi::MemoryMapHolder;
use crate::x86::write_cr3;
use crate::x86::PageAttr;
use crate::x86::PAGE_SIZE;
use crate::x86::PML4;
use alloc::boxed::Box;
use core::cmp::max;

pub fn init_basic_runtime(
    image_handle: EfiHandle,
    system_table: &EfiSystemTable,
) -> MemoryMapHolder {
    let mut memory_map = MemoryMapHolder::new();
    exit_from_efi_boot_services(image_handle, system_table, &mut memory_map);
    ALLOCATOR.init_with_mmap(&memory_map);
    memory_map
}

pub fn init_paging(memory_map: &MemoryMapHolder) {
    let mut table = PML4::new();
    let mut end_of_mem = 0x1_0000_0000u64; // 4GB
    for e in memory_map.iter() {
        match e.memory_type() {
            CONVENTIONAL_MEMORY | LOADER_CORE | LOADER_DATA => {
                end_of_mem = max(
                    end_of_mem,
                    e.physical_start() as u64 + e.number_of_pages() as u64 * (PAGE_SIZE as u64),
                );
            }
            _ => (),
        }
    }
    table
        .create_mapping(0, end_of_mem, 0, PageAttr::ReadWriteKernel)
        .expect("Failed to create mapping");
    unsafe {
        write_cr3(Box::into_raw(table));
    }
}
