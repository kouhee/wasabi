#![no_std]
#![no_main]
#![feature(offset_of)]

use core::panic::PanicInfo;
use core::time::Duration;
use wasabi::error;
use wasabi::executor::Executor;
use wasabi::executor::Task;
use wasabi::executor::TimeoutFuture;
use wasabi::hpet::global_timestamp;
use wasabi::info;
use wasabi::init::init_allocator;
use wasabi::init::init_basic_runtime;
use wasabi::init::init_display;
use wasabi::init::init_hpet;
use wasabi::init::init_paging;
use wasabi::print::hexdump;
use wasabi::print::set_global_vram;
use wasabi::println;
use wasabi::qemu::exit_qemu;
use wasabi::qemu::QemuExitCode;
use wasabi::uefi::init_vram;
use wasabi::uefi::locate_loaded_image_protocol;
use wasabi::uefi::EfiHandle;
use wasabi::uefi::EfiSystemTable;
//use wasabi::uefi::VramTextWriter;
use wasabi::warn;
// use wasabi::x86::hlt;
use wasabi::x86::init_exceptions;

#[no_mangle]
fn efi_main(image_handle: EfiHandle, efi_system_table: &EfiSystemTable) {
    println!("Booting wasabiOS...");
    println!("image_handle: {:#018X}", image_handle);
    println!("efi_system_table: {:#p}", efi_system_table);
    let loaded_image_protocol = locate_loaded_image_protocol(image_handle, efi_system_table)
        .expect("Failed to locate loaded image protocol");
    println!("image_base: {:#018X}", loaded_image_protocol.image_base);
    println!("image_size: {:#018X}", loaded_image_protocol.image_size);
    info!("info");
    warn!("warn");
    error!("error");
    hexdump(efi_system_table);
    // Print boot services pointer and a few entries (smaller dump)
    println!(
        "boot_services ptr: {:#018X}",
        efi_system_table.boot_services as *const _ as usize
    );
    println!("before init_vram");
    unsafe {
        let p = efi_system_table.boot_services as *const _ as *const u8 as *const u64;
        for i in 0..8 {
            println!("bs[{}] = {:#018X}", i, *p.add(i));
        }
    }
    let mut vram = match init_vram(efi_system_table) {
        Ok(v) => {
            println!("init_vram ok");
            v
        }
        Err(e) => {
            println!("init_vram failed: {e}");
            exit_qemu(QemuExitCode::Fail);
        }
    };
    init_display(&mut vram);
    set_global_vram(vram);
    println!("after init_vram");
    let acpi = efi_system_table
        .acpi_table()
        .expect("Failed to get ACPI table");

    let memory_map = init_basic_runtime(image_handle, efi_system_table);
    init_allocator(&memory_map);
    // exit_from_efi_boot_services(image_handle, efi_system_table, &mut memory_map);
    info!("Hello, Non-UEFI world!");

    let (_gdt, _idt) = init_exceptions();
    init_paging(&memory_map);
    init_hpet(acpi);
    let t0 = global_timestamp();
    let task1 = Task::new(async move {
        for i in 100..103 {
            info!("{i} hpet.main_counter = {:?}", global_timestamp() - t0);
            TimeoutFuture::new(Duration::from_secs(1)).await;
        }
        Ok(())
    });
    let task2 = Task::new(async move {
        for i in 200..203 {
            info!("{i} hpet.main_counter = {:?}", global_timestamp() - t0);
            TimeoutFuture::new(Duration::from_secs(2)).await;
        }
        Ok(())
    });
    let mut executor = Executor::new();
    executor.enqueue(task1);
    executor.enqueue(task2);
    Executor::run(executor);
    // info!("Reading from memory address 0...(again)");
    // #[allow(clippy::zero_ptr)]
    // #[allow(deref_nullptr)]
    // let value_at_zero = unsafe { *(0 as *const u8) };
    // info!("value_at_zero = {value_at_zero}");

    // loop {
    //     hlt();
    // }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("Panic: {info:?}");
    exit_qemu(QemuExitCode::Fail);
}
