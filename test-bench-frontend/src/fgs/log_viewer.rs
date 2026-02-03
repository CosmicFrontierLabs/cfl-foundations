//! WebSocket-based log viewer component.
//!
//! A reusable Yew component that connects to a `/logs` WebSocket endpoint
//! and displays streaming log messages with automatic scrolling.

use gloo_net::websocket::{futures::WebSocket, Message};
use shared_wasm::{LogEntry, LogLevel};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

/// Maximum number of log entries to keep in memory.
const MAX_LOG_ENTRIES: usize = 500;

/// Props for the LogViewer component.
#[derive(Properties, PartialEq)]
pub struct LogViewerProps {
    /// WebSocket URL to connect to (e.g., "ws://localhost:3000/logs" or "/logs" for same-origin)
    #[prop_or("/logs".into())]
    pub ws_url: AttrValue,
    /// Maximum height of the log container (CSS value)
    #[prop_or("300px".into())]
    pub max_height: AttrValue,
    /// Whether to show timestamps
    #[prop_or(true)]
    pub show_timestamps: bool,
    /// Whether to show log targets (module paths)
    #[prop_or(true)]
    pub show_targets: bool,
    /// Minimum log level to display
    #[prop_or(LogLevel::Debug)]
    pub min_level: LogLevel,
}

/// Internal message types for the LogViewer component.
pub enum LogViewerMsg {
    /// New log entry received from WebSocket
    LogReceived(LogEntry),
    /// WebSocket connection status changed
    ConnectionStatus(bool),
    /// Clear all logs
    Clear,
    /// Toggle auto-scroll
    ToggleAutoScroll,
    /// Reconnect to WebSocket
    Reconnect,
}

/// A reusable log viewer component that streams logs via WebSocket.
pub struct LogViewer {
    logs: Vec<LogEntry>,
    connected: bool,
    auto_scroll: bool,
    container_ref: NodeRef,
}

impl Component for LogViewer {
    type Message = LogViewerMsg;
    type Properties = LogViewerProps;

    fn create(ctx: &Context<Self>) -> Self {
        // Start WebSocket connection
        let link = ctx.link().clone();
        let ws_url = ctx.props().ws_url.clone();
        spawn_local(async move {
            connect_websocket(ws_url.to_string(), link).await;
        });

        Self {
            logs: Vec::new(),
            connected: false,
            auto_scroll: true,
            container_ref: NodeRef::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            LogViewerMsg::LogReceived(entry) => {
                // Filter by minimum level
                if !should_display(&entry.level, &ctx.props().min_level) {
                    return false;
                }

                self.logs.push(entry);

                // Trim old entries
                if self.logs.len() > MAX_LOG_ENTRIES {
                    self.logs.remove(0);
                }

                // Auto-scroll to bottom
                if self.auto_scroll {
                    if let Some(container) = self.container_ref.cast::<web_sys::Element>() {
                        container.set_scroll_top(container.scroll_height());
                    }
                }

                true
            }
            LogViewerMsg::ConnectionStatus(connected) => {
                self.connected = connected;
                true
            }
            LogViewerMsg::Clear => {
                self.logs.clear();
                true
            }
            LogViewerMsg::ToggleAutoScroll => {
                self.auto_scroll = !self.auto_scroll;
                true
            }
            LogViewerMsg::Reconnect => {
                let link = ctx.link().clone();
                let ws_url = ctx.props().ws_url.clone();
                spawn_local(async move {
                    connect_websocket(ws_url.to_string(), link).await;
                });
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let props = ctx.props();
        let link = ctx.link();

        let status_color = if self.connected { "#00ff00" } else { "#ff4444" };
        let status_text = if self.connected {
            "Connected"
        } else {
            "Disconnected"
        };

        let on_clear = link.callback(|_| LogViewerMsg::Clear);
        let on_toggle_scroll = link.callback(|_| LogViewerMsg::ToggleAutoScroll);
        let on_reconnect = link.callback(|_| LogViewerMsg::Reconnect);

        let scroll_btn_style = if self.auto_scroll {
            "background: #00ff00; color: #000;"
        } else {
            "background: #333; color: #888;"
        };

        html! {
            <div class="log-viewer" style="font-family: 'Courier New', monospace; font-size: 0.75em; background: #0a0a0a; border: 1px solid #333; padding: 5px;">
                // Header with controls
                <div style="display: flex; justify-content: space-between; align-items: center; padding: 3px 5px; border-bottom: 1px solid #333; margin-bottom: 5px;">
                    <div>
                        <span style={format!("color: {status_color}; font-weight: bold;")}>
                            {"[LOG] "}{status_text}
                        </span>
                        <span style="color: #666; margin-left: 10px;">
                            {format!("{} entries", self.logs.len())}
                        </span>
                    </div>
                    <div style="display: flex; gap: 5px;">
                        <button
                            onclick={on_toggle_scroll}
                            style={format!("border: none; padding: 2px 6px; cursor: pointer; font-size: 0.9em; {scroll_btn_style}")}
                        >
                            {"Auto-scroll"}
                        </button>
                        <button
                            onclick={on_clear}
                            style="background: #333; color: #fff; border: none; padding: 2px 6px; cursor: pointer; font-size: 0.9em;"
                        >
                            {"Clear"}
                        </button>
                        if !self.connected {
                            <button
                                onclick={on_reconnect}
                                style="background: #ff6600; color: #000; border: none; padding: 2px 6px; cursor: pointer; font-size: 0.9em;"
                            >
                                {"Reconnect"}
                            </button>
                        }
                    </div>
                </div>

                // Log container
                <div
                    ref={self.container_ref.clone()}
                    style={format!("max-height: {}; overflow-y: auto; padding: 5px;", props.max_height)}
                >
                    { for self.logs.iter().map(|entry| render_log_entry(entry, props)) }
                    if self.logs.is_empty() {
                        <div style="color: #666; text-align: center; padding: 20px;">
                            {"No logs yet..."}
                        </div>
                    }
                </div>
            </div>
        }
    }
}

/// Render a single log entry.
fn render_log_entry(entry: &LogEntry, props: &LogViewerProps) -> Html {
    let level_color = entry.level.color();

    // Format timestamp as HH:MM:SS.mmm
    let timestamp = if props.show_timestamps {
        let total_secs = entry.timestamp_ms / 1000;
        let hours = (total_secs / 3600) % 24;
        let mins = (total_secs / 60) % 60;
        let secs = total_secs % 60;
        let millis = entry.timestamp_ms % 1000;
        format!("{hours:02}:{mins:02}:{secs:02}.{millis:03} ")
    } else {
        String::new()
    };

    // Truncate target to last component
    let target = if props.show_targets {
        let short_target = entry.target.rsplit("::").next().unwrap_or(&entry.target);
        format!("[{short_target}] ")
    } else {
        String::new()
    };

    html! {
        <div style="white-space: pre-wrap; word-break: break-word; margin: 1px 0;">
            if props.show_timestamps {
                <span style="color: #666;">{timestamp}</span>
            }
            <span style={format!("color: {level_color}; font-weight: bold;")}>
                {format!("{:5} ", entry.level)}
            </span>
            if props.show_targets {
                <span style="color: #888;">{target}</span>
            }
            <span style="color: #ddd;">{&entry.message}</span>
        </div>
    }
}

/// Check if a log level should be displayed given the minimum level.
fn should_display(level: &LogLevel, min_level: &LogLevel) -> bool {
    let level_rank = match level {
        LogLevel::Trace => 0,
        LogLevel::Debug => 1,
        LogLevel::Info => 2,
        LogLevel::Warn => 3,
        LogLevel::Error => 4,
    };
    let min_rank = match min_level {
        LogLevel::Trace => 0,
        LogLevel::Debug => 1,
        LogLevel::Info => 2,
        LogLevel::Warn => 3,
        LogLevel::Error => 4,
    };
    level_rank >= min_rank
}

/// Connect to WebSocket and stream log entries.
async fn connect_websocket(url: String, link: yew::html::Scope<LogViewer>) {
    use futures_util::StreamExt;

    // Build full WebSocket URL
    let ws_url = if url.starts_with("ws://") || url.starts_with("wss://") {
        url
    } else {
        // Relative URL - build from current location
        let window = web_sys::window().expect("no window");
        let location = window.location();
        let protocol = if location.protocol().unwrap_or_default() == "https:" {
            "wss:"
        } else {
            "ws:"
        };
        let host = location.host().unwrap_or_default();
        format!("{protocol}//{host}{url}")
    };

    match WebSocket::open(&ws_url) {
        Ok(ws) => {
            link.send_message(LogViewerMsg::ConnectionStatus(true));

            let (_, mut read) = ws.split();

            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        // Try to parse as LogEntry
                        if let Ok(entry) = serde_json::from_str::<LogEntry>(&text) {
                            link.send_message(LogViewerMsg::LogReceived(entry));
                        }
                    }
                    Ok(Message::Bytes(_)) => {
                        // Binary messages not used for logs
                    }
                    Err(e) => {
                        web_sys::console::log_1(&format!("WebSocket error: {e:?}").into());
                        break;
                    }
                }
            }

            link.send_message(LogViewerMsg::ConnectionStatus(false));
        }
        Err(e) => {
            web_sys::console::log_1(&format!("Failed to connect WebSocket: {e:?}").into());
            link.send_message(LogViewerMsg::ConnectionStatus(false));
        }
    }
}
