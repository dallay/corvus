use crate::tui::app::{ActiveView, ViewData};
use crossterm::event::{self, Event, KeyEvent};
use std::time::Duration;

#[derive(Debug)]
pub enum RuntimeEvent {
    Key(KeyEvent),
    Tick,
    ViewLoaded { view: ActiveView, data: ViewData },
    ViewFailed { view: ActiveView, message: String },
}

pub fn poll_terminal_event(timeout: Duration) -> Result<Option<RuntimeEvent>, String> {
    if event::poll(timeout).map_err(|err| err.to_string())? {
        match event::read().map_err(|err| err.to_string())? {
            Event::Key(key) => Ok(Some(RuntimeEvent::Key(key))),
            _ => Ok(None),
        }
    } else {
        Ok(Some(RuntimeEvent::Tick))
    }
}
