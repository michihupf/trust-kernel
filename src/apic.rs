use crate::bitmask;

fn has_local_apic() -> bool {
    // Safety: CPUID.1h is available
    let cpuid = unsafe { core::arch::x86_64::__cpuid(0x1) };
    cpuid.edx & bitmask!(9) != 0
}
