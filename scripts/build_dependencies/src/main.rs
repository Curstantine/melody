mod constants;
mod utils;

#[cfg_attr(unix, path = "./platforms/unix.rs")]
#[cfg_attr(windows, path = "./platforms/windows.rs")]
mod platform;

fn main() {
	println!("Hello, world!");
	platform::run().unwrap();
}
