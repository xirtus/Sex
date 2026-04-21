use crate::memory;

// Placeholder for PDX process spawning. In a real kernel, this would
// involve more complex logic to load and execute a new process domain.
// For now, we'll just log that we're attempting to spawn.
#[allow(unused_variables)]
fn pdx_spawn(name: &str) -> Result<(), &'static str> {
    // In a real scenario, this would map memory, load the executable,
    // and set up the process context.
    // For demonstration, we'll print a message.
    // println!("Attempting to spawn PD: {}", name);
    Ok(())
}

pub fn kernel_init() {
    memory::init();

    // Initialize sexdisplay. We assume it's either already running or
    // its initialization is handled by lower-level boot code.
    // If sexdisplay needed explicit initialization from here, we'd add:
    // sexdisplay::init();

    // Auto-start the silk-shell PD after sexdisplay is ready.
    // This is the critical change to launch the Silk shell.
    if let Err(e) = pdx_spawn("silk-shell") {
        // Handle error, e.g., log it or halt the kernel
        // println!("Failed to spawn silk-shell: {}", e);
    }
}
