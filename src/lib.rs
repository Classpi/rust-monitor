use crate::eventloop::{Event, EventLoop};
#[derive(Debug)]
pub struct AppError(String);
impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for AppError {}
impl From<windows::core::Error> for AppError {
    fn from(value: windows::core::Error) -> Self {
        Self(value.message())
    }
}
pub trait ApplicationEventHandler {
    fn resumed(&mut self, eventloop: &EventLoop);
    fn event(&mut self, eventloop: &EventLoop, event: Event);
}
pub mod app;
pub mod eventloop;
pub mod timer;
pub mod window;
pub mod render;
// pub mod my_error;