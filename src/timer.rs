use std::{
    sync::{Arc, Mutex, mpsc::Sender},
    thread,
    time::{Duration, Instant},
};

use crate::eventloop::Event;

pub struct TimerManager {
    /// Event, Duration, Start_time, Is_active
    timers: Arc<Mutex<Vec<(Event, Duration, Instant, bool)>>>,
}

impl TimerManager {
    pub fn new(sx: Sender<Event>) -> Self {
        let timers = Arc::new(Mutex::new(Vec::new()));
        let timers_clone = timers.clone();
        let sx_clone = sx.clone();

        thread::Builder::new()
            .name("timer_thread".to_string())
            .spawn(move || {
                loop {
                    let now = Instant::now();

                    let (events_to_fire, next_min_sleep) = {
                        let mut timers = timers_clone.lock().unwrap();
                        let mut events = Vec::new();
                        let mut min_sleep = Duration::from_secs(1);

                        for (event, interval, start_time, active) in timers.iter_mut() {
                            if *active {
                                let elapsed = now.duration_since(*start_time);
                                if elapsed >= *interval {
                                    events.push(*event);
                                    *start_time = now;
                                    min_sleep = min_sleep.min(*interval);
                                } else {
                                    let remaining = *interval - elapsed;
                                    min_sleep = min_sleep.min(remaining);
                                }
                            }
                        }
                        (events, min_sleep)
                    };

                    for event in events_to_fire {
                        if sx_clone.send(event).is_err() {
                            break;
                        }
                    }

                    let sleep_duration = next_min_sleep.min(Duration::from_millis(330));
                    thread::sleep(sleep_duration);
                }
            })
            .expect("Failed to spawn timer thread");

        Self { timers }
    }

    pub fn start_timer(&self, event: Event, duration: Duration) {
        let mut timers = self.timers.lock().unwrap();
        let start_time = Instant::now();

        if let Some(existing) = timers.iter_mut().find(|(e, _, _, _)| *e == event) {
            existing.1 = duration;
            existing.2 = start_time;
            existing.3 = true;
        } else {
            timers.push((event, duration, start_time, true));
        }
    }

    pub fn stop_timer(&self, event: Event) {
        let mut timers = self.timers.lock().unwrap();
        if let Some(timer) = timers.iter_mut().find(|(e, _, _, _)| *e == event) {
            timer.3 = false; // 设置为不活跃
        }
    }

    pub fn remove_timer(&self, event: Event) {
        let mut timers = self.timers.lock().unwrap();
        timers.retain(|(e, _, _, _)| *e != event);
    }
}
