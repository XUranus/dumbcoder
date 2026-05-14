use anyhow::Result;
use crossterm::event::{self, Event};
use std::time::Duration;
use tokio::sync::mpsc;

use super::app::AppEvent;

pub struct EventHandler {
    tick_rate: Duration,
    model_rx: mpsc::Receiver<anyhow::Result<String>>,
}

impl EventHandler {
    pub fn new(
        tick_rate: Duration,
        model_rx: mpsc::Receiver<anyhow::Result<String>>,
    ) -> Self {
        Self { tick_rate, model_rx }
    }

    pub async fn next(&mut self) -> Result<AppEvent> {
        if let Ok(result) = self.model_rx.try_recv() {
            return Ok(AppEvent::ModelResult(result));
        }

        if event::poll(self.tick_rate)? {
            if let Event::Key(key) = event::read()? {
                return Ok(AppEvent::Key(key));
            }
        }

        if let Ok(result) = self.model_rx.try_recv() {
            return Ok(AppEvent::ModelResult(result));
        }

        Ok(AppEvent::Tick)
    }
}
