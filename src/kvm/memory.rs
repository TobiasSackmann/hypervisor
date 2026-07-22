use anyhow::{Context, Result};
use kvm_bindings::{KVM_MEM_LOG_DIRTY_PAGES, kvm_userspace_memory_region};
use kvm_ioctls::VmFd;
use vm_memory::{Bytes, GuestAddress, GuestMemory, GuestMemoryMmap};

/// Userspace memory region that is mapped into the KVM guest physical address space.
pub struct GuestRam {
    memory: GuestMemoryMmap<()>,
    start: u64,
    size: usize,
}

impl GuestRam {
    /// Allocates a contiguous guest RAM region starting at `start`.
    pub fn new(start: u64, size: usize) -> Result<Self> {
        let memory = GuestMemoryMmap::from_ranges(&[(GuestAddress(start), size)])
            .context("allocate guest memory")?;

        Ok(Self {
            memory,
            start,
            size,
        })
    }

    /// Writes bytes to the beginning of this guest RAM region.
    pub fn write_at_start(&self, bytes: &[u8]) -> Result<()> {
        self.memory
            .write_slice(bytes, GuestAddress(self.start))
            .context("write guest code into guest memory")
    }

    /// Registers this userspace memory with KVM so the guest can access it.
    pub fn register_with_kvm(&self, vm: &VmFd, slot: u32) -> Result<()> {
        let host_addr = self
            .memory
            .get_host_address(GuestAddress(self.start))
            .context("get host address for guest memory")?;

        let region = kvm_userspace_memory_region {
            slot,
            guest_phys_addr: self.start,
            memory_size: self.size as u64,
            userspace_addr: host_addr as u64,
            flags: KVM_MEM_LOG_DIRTY_PAGES,
        };

        unsafe {
            vm.set_user_memory_region(region)
                .context("register guest memory with KVM")?;
        }

        Ok(())
    }
}
