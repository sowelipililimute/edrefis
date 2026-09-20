use std::time::Duration;

#[cfg(not(target_family = "wasm"))]
use std::time::Instant as StandardInstant;

#[cfg(not(target_family = "wasm"))]
pub struct Instant(StandardInstant);

#[cfg(not(target_family = "wasm"))]
impl Instant {
    pub fn now() -> Instant {
        Instant(StandardInstant::now())
    }

    pub fn elapsed(&self) -> Duration {
        self.0.elapsed()
    }
}

#[cfg(target_family = "wasm")]
use web_sys::window;

#[cfg(target_family = "wasm")]
pub struct Instant(f64);

#[cfg(target_family = "wasm")]
impl Instant {
    pub fn now() -> Instant {
        let performance = window().unwrap().performance().unwrap();

        Instant(performance.now())
    }

    pub fn elapsed(&self) -> Duration {
        let performance = window().unwrap().performance().unwrap();

        Duration::from_millis((performance.now() - self.0) as u64)
    }
}
