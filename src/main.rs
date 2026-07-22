mod guest;
mod kvm;

fn main() -> anyhow::Result<()> {
    kvm::vm::run_hello_vm()
}
