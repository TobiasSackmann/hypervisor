mod guest;
mod kvm;

fn main() -> anyhow::Result<()> {
    let message = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "Hallo aus meiner VM!\n".to_string());

    println!("Gewählte Nachricht: {message}");

    kvm::vm::run_hello_vm(message)
}
