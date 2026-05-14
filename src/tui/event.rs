use anyhow::Result;
use crossterm::event::{self, Event, KeyEvent};
use std::time::Duration;
use tokio::sync::mpsc;

use super::app::AppEvent;

pub struct EventHandler {
    tick_rate: Duration,
    model_rx: mpsc::Receiver<anyhow::Result<super::app::ModelResponse>>,
}

impl EventHandler {
    pub fn new(
        tick_rate: Duration,
        model_rx: mpsc::Receiver<anyhow::Result<super::app::ModelResponse>>,
    ) -> Self {
        Self { tick_rate, model_rx }
    }

    pub async fn next(&mut self) -> Result<AppEvent> {
        // Check for model results first (non-blocking)
        if let Ok(result) = self.model_rx.try_recv() {
            return Ok(AppEvent::ModelResult(result));
        }

        // Poll for keyboard events with timeout
        if event::poll(self.tick_rate)? {
            match event::read()? {
                Event::Key(key) => return Ok(AppEvent::Key(key)),
                _ => {}
            }
        }

        // Check channel again after poll
        if let Ok(result) = self.model_rx.try_recv() {
            return Ok(AppEvent::ModelResult(result));
        }

        Ok(AppEvent::Tick)
    }
}
