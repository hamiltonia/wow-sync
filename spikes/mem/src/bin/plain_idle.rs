// Reference: plain non-GPUI Rust process, idle.
fn main() {
    println!("plain_idle pid={}", std::process::id());
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}
