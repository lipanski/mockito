use std::time::Duration;

fn main() {
    mockito::start();

    loop {
        std::thread::sleep(Duration::from_secs(1))
    }
}
