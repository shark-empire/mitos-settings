//! A minimal publish/subscribe bus. Any part of the app — the interactive
//! navigator today, a future status-bar process tomorrow — can subscribe to
//! be told whenever a setting changes, without `SettingsManager` needing to
//! know who's listening.

use crate::settings::value::Value;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub enum Event {
    SettingChanged { key: String, value: Value },
    System(String),
}

pub struct EventBus {
    subscribers: Mutex<Vec<Sender<Event>>>,
}

impl EventBus {
    pub fn new() -> Self {
        EventBus { subscribers: Mutex::new(Vec::new()) }
    }

    /// Returns a receiver that will get every event published from this
    /// point on. Dropping the receiver unsubscribes automatically (the next
    /// `publish` notices the closed channel and prunes it).
    pub fn subscribe(&self) -> Receiver<Event> {
        let (tx, rx) = channel();
        self.subscribers.lock().unwrap_or_else(|e| e.into_inner()).push(tx);
        rx
    }

    pub fn publish(&self, event: Event) {
        let mut subs = self.subscribers.lock().unwrap_or_else(|e| e.into_inner());
        subs.retain(|tx| tx.send(event.clone()).is_ok());
    }

    pub fn subscriber_count(&self) -> usize {
        self.subscribers.lock().unwrap_or_else(|e| e.into_inner()).len()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscribers_receive_published_events() {
        let bus = EventBus::new();
        let rx = bus.subscribe();
        bus.publish(Event::SettingChanged { key: "sound.volume".into(), value: Value::Int(50) });
        let event = rx.recv().unwrap();
        match event {
            Event::SettingChanged { key, value } => {
                assert_eq!(key, "sound.volume");
                assert_eq!(value, Value::Int(50));
            }
            _ => panic!("wrong event variant"),
        }
    }

    #[test]
    fn dropped_subscribers_are_pruned() {
        let bus = EventBus::new();
        {
            let _rx = bus.subscribe();
            assert_eq!(bus.subscriber_count(), 1);
        }
        bus.publish(Event::System("tick".into()));
        assert_eq!(bus.subscriber_count(), 0);
    }
}
