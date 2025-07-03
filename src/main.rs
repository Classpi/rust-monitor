use rust_zooming_cat_v2::app::App;
use rust_zooming_cat_v2::eventloop::*;
use rust_zooming_cat_v2::timer::TimerManager;

fn main() {
    env_logger::builder().filter_level(log::LevelFilter::Debug).init();
    let (mut eventloop, sender) = EventLoop::new();
    let timer_manager = TimerManager::new(sender);
    let mut app: App = App {
        window: None,
        render: None,
        timer_manager: Some(timer_manager),
    };

    eventloop.run_app(&mut app);
}
