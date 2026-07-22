use anyhow::{Context, Result};
use kvm_bindings::{KVM_MEM_LOG_DIRTY_PAGES, kvm_userspace_memory_region};
use kvm_ioctls::VmFd;
use vm_memory::{Bytes, GuestAddress, GuestMemory, GuestMemoryMmap};

pub struct GuestRam {
    memory: GuestMemoryMmap<()>,
    start: u64,
    size: usize,
}

impl GuestRam {
    pub fn new(start: u64, size: usize) -> Result<Self> {
        let memory = GuestMemoryMmap::from_ranges(&[(GuestAddress(start), size)])
            .context("allocate guest memory")?;

        Ok(Self {
            memory,
            start,
            size,
        })
    }

    pub fn write_at_start(&self, bytes: &[u8]) -> Result<()> {
        self.memory
            .write_slice(bytes, GuestAddress(self.start))
            .context("write guest code into guest memory")
    }

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
