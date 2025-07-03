use crate::ApplicationEventHandler;
use std::sync::mpsc::{Receiver, Sender};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    AppCreate,
    AppDestory,
    AppRenderChange,
    Paint,
    Resize(u32, u32),
    Close,
    KeyDown(u32),
    MouseMove(i32, i32),
}

pub struct EventLoop {
    pub event_sender: Sender<Event>,
    pub event_receiver: Receiver<Event>,
}
impl EventLoop {
    pub fn new() -> (Self, Sender<Event>) {
        let (sx, rx) = std::sync::mpsc::channel();
        (
            Self {
                event_sender: sx.clone(),
                event_receiver: rx,
            },
            sx,
        )
    }

    pub fn run_app<T: ApplicationEventHandler>(&mut self, app: &mut T) {
        app.resumed(&self);

        loop {
            if let Ok(event) = self.event_receiver.recv() {
                app.event(&self, event);
            }
        }
    }
}
