use std::{path::Path, time::Duration};

use log::debug;

use super::ApplicationEventHandler;
use crate::{
    eventloop::{Event, EventLoop},
    render::{Render, dx_render::DxRender},
    timer::TimerManager,
    window::Window,
};
#[derive(Default)]
pub struct App {
    pub window: Option<Window>,
    pub render: Option<Box<dyn Render>>,
    pub timer_manager: Option<TimerManager>,
}

impl App {
    pub fn into_with_render<R: Render + 'static>(self, render: R) -> App {
        App {
            window: self.window,
            render: Some(Box::new(render)),
            timer_manager: self.timer_manager,
        }
    }
}

impl ApplicationEventHandler for App {
    fn resumed(&mut self, event_loop: &crate::eventloop::EventLoop) {
        self.window = Some(Window::init(event_loop).unwrap());
        self.timer_manager.as_mut().unwrap().start_timer(crate::eventloop::Event::Paint, Duration::from_millis(20));
        let render = DxRender::new(self.window.as_ref().unwrap().hwnd);
        let base = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let abs_path = Path::new(&base).join("resources/gif/cat-rainbow.gif");
        let path = abs_path.to_str().unwrap();
        let mut render = render.unwrap();
        let _ = render.load_src_data(path);
        self.render = Some(Box::new(render));
    }

    fn event(&mut self, event_loop: &EventLoop, event: Event) {
        match event {
            Event::Paint => {
                if let Some(ref mut render) = self.render {
                    let _ = render.render_frame().and_then(|()| render.next_frame());
                }
            }
            _ => {
                debug!("{:?}", event);
            }
        }
    }
}
