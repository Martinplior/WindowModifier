#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::any::Any;

use app::MainApp;

mod app;

mod message_dialog;
mod window_modifier;

fn graceful_run<R>(
    f: impl FnOnce() -> R + std::panic::UnwindSafe,
) -> Result<R, Box<dyn Any + Send + 'static>> {
    std::panic::catch_unwind(f).map_err(|err| {
        let message = if let Some(str) = err.downcast_ref::<&str>() {
            str.to_string()
        } else if let Some(string) = err.downcast_ref::<String>() {
            string.clone()
        } else {
            format!("{:?}", err)
        };
        message_dialog::error(&message).show();
        err
    })
}

fn main() {
    let _ = graceful_run(|| MainApp::new().run());
}
