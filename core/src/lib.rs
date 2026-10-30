pub mod protocol {
    include!(concat!(env!("OUT_DIR"), "/merci.rs"));
}
pub mod engine;
mod ffi;
pub mod game;
pub mod rating;
