//! The server side of `LibraryEvents`: one broadcast channel, fanned out to
//! every connected browser as server-sent events.

use std::convert::Infallible;
use std::time::Duration;

use axum::response::sse::{Event, KeepAlive, Sse};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};

use command_center_core::db::settings::AppSettings;
use command_center_core::events::{LibraryEvents, LIBRARY_CHANGED, SETTINGS_CHANGED};

/// Enough room that a browser which stalls briefly catches up rather than
/// falling behind. A client that falls further behind than this is told to
/// reload the library instead, which is the honest recovery.
const CHANNEL_CAPACITY: usize = 64;

/// Proxies and load balancers close an idle connection, so the feed sends a
/// comment often enough to stay open.
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(15);

/// One of the two notifications, ready to be encoded for a browser.
#[derive(Debug, Clone)]
pub enum ServerEvent {
    LibraryChanged,
    SettingsChanged(Box<AppSettings>),
}

/// The listener the core writes through, and the source every feed subscribes
/// to.
pub struct BroadcastEvents {
    sender: broadcast::Sender<ServerEvent>,
}

impl Default for BroadcastEvents {
    fn default() -> Self {
        Self {
            sender: broadcast::channel(CHANNEL_CAPACITY).0,
        }
    }
}

impl BroadcastEvents {
    pub fn subscribe(&self) -> broadcast::Receiver<ServerEvent> {
        self.sender.subscribe()
    }
}

impl LibraryEvents for BroadcastEvents {
    fn library_changed(&self) {
        // An error here only means nobody is connected, which is not a reason
        // to fail the write that just succeeded.
        let _ = self.sender.send(ServerEvent::LibraryChanged);
    }

    fn settings_changed(&self, settings: &AppSettings) {
        let _ = self
            .sender
            .send(ServerEvent::SettingsChanged(Box::new(settings.clone())));
    }
}

/// Encodes one notification under the same event name the frontend already
/// listens for, so the browser handler is the one the desktop app uses.
fn encode(event: ServerEvent) -> Event {
    match event {
        ServerEvent::LibraryChanged => Event::default().event(LIBRARY_CHANGED).data("null"),
        ServerEvent::SettingsChanged(settings) => Event::default()
            .event(SETTINGS_CHANGED)
            .json_data(&*settings)
            // Settings that will not encode cannot be sent, but the client
            // still has to know they changed and can reload them itself.
            .unwrap_or_else(|_| Event::default().event(SETTINGS_CHANGED).data("null")),
    }
}

/// The live feed. A client that has fallen too far behind is sent a plain
/// library-changed, which makes it refetch: safer than pretending it is
/// current.
pub fn feed(
    receiver: broadcast::Receiver<ServerEvent>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = BroadcastStream::new(receiver).map(|received| {
        Ok(match received {
            Ok(event) => encode(event),
            Err(_lagged) => encode(ServerEvent::LibraryChanged),
        })
    });

    Sse::new(stream).keep_alive(KeepAlive::new().interval(KEEP_ALIVE_INTERVAL))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_write_reaches_everyone_who_is_listening() {
        let events = BroadcastEvents::default();
        let mut first = events.subscribe();
        let mut second = events.subscribe();

        events.library_changed();

        assert!(matches!(
            first.recv().await.unwrap(),
            ServerEvent::LibraryChanged
        ));
        assert!(matches!(
            second.recv().await.unwrap(),
            ServerEvent::LibraryChanged
        ));
    }

    #[test]
    fn writing_with_nobody_connected_is_not_a_failure() {
        let events = BroadcastEvents::default();

        // No subscribers at all. This must not panic or block.
        events.library_changed();
        events.settings_changed(&AppSettings::default());
    }

    #[tokio::test]
    async fn saved_settings_travel_with_the_notification() {
        let events = BroadcastEvents::default();
        let mut feed = events.subscribe();

        events.settings_changed(&AppSettings {
            theme: "light".into(),
            ..AppSettings::default()
        });

        match feed.recv().await.unwrap() {
            ServerEvent::SettingsChanged(settings) => assert_eq!(settings.theme, "light"),
            other => panic!("expected settings, got {other:?}"),
        }
    }
}
