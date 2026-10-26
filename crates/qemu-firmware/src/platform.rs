use core::arch::asm;
use core::fmt;

pub struct Console;

impl fmt::Write for Console {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for byte in text.bytes() {
            // SAFETY: QEMU TCG semihosting is enabled. SYS_WRITEC reads one byte
            // from the live stack address in x1 before returning. x0/x1 are
            // declared clobbered; no nomem promise is made about the host read.
            unsafe {
                asm!("hlt #0xf000",
                    inout("x0") 0x03usize => _,
                    inout("x1") &byte as *const u8 => _,
                    options(nostack));
            }
        }
        Ok(())
    }
}

pub fn exit(status: usize) -> ! {
    // Arm A64 SYS_EXIT takes { ADP_Stopped_ApplicationExit, status }.
    let arguments = [0x20026usize, status];
    // SAFETY: QEMU's A64 semihosting handler consumes this valid two-word stack
    // block synchronously. SYS_EXIT terminates the guest; if a debugger resumes
    // it, the fallback loop never returns into code assuming a successful exit.
    unsafe {
        asm!("hlt #0xf000",
            inout("x0") 0x18usize => _,
            inout("x1") arguments.as_ptr() => _,
            options(nostack));
    }
    loop {
        core::hint::spin_loop();
    }
}
