use sex_pdx::{PdxClient, MessageType};

pub fn spawn_signal_trampoline(pdx: PdxClient) {
    std::thread::spawn(move || {
        let mut ring = pdx.ring();               // zero-copy ring view
        loop {
            if let Some(msg) = ring.dequeue::<MessageType>() {
                if let MessageType::Signal(sig) = msg {
                    unsafe { crate::posix::invoke_user_handler(sig) };
                }
            }
            ring.wait();                         // existing futex-style park
        }
    });
}
