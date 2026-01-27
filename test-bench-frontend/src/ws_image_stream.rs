//! WebSocket-based image streaming component.
//!
//! This module provides a Yew component that displays camera frames received
//! over WebSocket with proper connection lifecycle handling. When the connection
//! closes (e.g., due to frame size change), the component automatically reconnects.

use gloo_net::websocket::{futures::WebSocket, Message};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Blob, Url};
use yew::prelude::*;

use futures_util::StreamExt;

/// Props for the WebSocket image stream component.
#[derive(Properties, PartialEq)]
pub struct WsImageStreamProps {
    /// Base URL for the WebSocket connection (e.g., "ws://localhost:3000")
    /// If not provided, derives from current page URL
    #[prop_or_default]
    pub base_url: Option<String>,

    /// CSS ID for the img element
    #[prop_or("camera-frame".to_string())]
    pub id: String,

    /// CSS class for the img element
    #[prop_or("image-frame".to_string())]
    pub class: String,

    /// CSS style for the img element
    #[prop_or("cursor: crosshair; touch-action: pinch-zoom; display: block;".to_string())]
    pub style: String,

    /// Callback when image is clicked (passes x, y relative to element)
    #[prop_or_default]
    pub onclick: Option<Callback<(i32, i32)>>,

    /// Callback when image is touched (passes x, y relative to element)
    #[prop_or_default]
    pub ontouchstart: Option<Callback<(i32, i32)>>,

    /// Callback when frame dimensions change (width, height)
    #[prop_or_default]
    pub on_size_change: Option<Callback<(u32, u32)>>,

    /// Callback for connection status changes
    #[prop_or_default]
    pub on_connection_change: Option<Callback<bool>>,
}

/// Internal state for the WebSocket stream component.
pub struct WsImageStream {
    /// Current image blob URL (revoked when replaced)
    current_blob_url: Option<String>,
    /// Current frame dimensions
    frame_size: Option<(u32, u32)>,
    /// Connection status
    connected: bool,
    /// Reconnect attempts
    reconnect_count: u32,
}

pub enum Msg {
    /// New frame received from WebSocket
    FrameReceived {
        blob_url: String,
        width: u32,
        height: u32,
    },
    /// WebSocket connection opened
    Connected,
    /// WebSocket connection closed
    Disconnected,
    /// WebSocket error
    Error(String),
}

impl Component for WsImageStream {
    type Message = Msg;
    type Properties = WsImageStreamProps;

    fn create(ctx: &Context<Self>) -> Self {
        // Start WebSocket connection
        start_ws_connection(ctx);

        Self {
            current_blob_url: None,
            frame_size: None,
            connected: false,
            reconnect_count: 0,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::FrameReceived {
                blob_url,
                width,
                height,
            } => {
                // Revoke old blob URL to prevent memory leak
                if let Some(old_url) = self.current_blob_url.take() {
                    let _ = Url::revoke_object_url(&old_url);
                }

                self.current_blob_url = Some(blob_url);

                // Check if frame size changed
                let new_size = (width, height);
                if self.frame_size != Some(new_size) {
                    self.frame_size = Some(new_size);
                    if let Some(ref cb) = ctx.props().on_size_change {
                        cb.emit(new_size);
                    }
                }

                true
            }
            Msg::Connected => {
                self.connected = true;
                self.reconnect_count = 0;
                if let Some(ref cb) = ctx.props().on_connection_change {
                    cb.emit(true);
                }
                false
            }
            Msg::Disconnected => {
                self.connected = false;
                if let Some(ref cb) = ctx.props().on_connection_change {
                    cb.emit(false);
                }

                // Schedule reconnect with exponential backoff
                let delay = calculate_reconnect_delay(self.reconnect_count);
                self.reconnect_count += 1;

                let link = ctx.link().clone();
                gloo_timers::callback::Timeout::new(delay, move || {
                    start_ws_connection_with_link(&link);
                })
                .forget();

                false
            }
            Msg::Error(_e) => {
                // Error logged during connection handling
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let props = ctx.props();

        let onclick = if let Some(ref cb) = props.onclick {
            let cb = cb.clone();
            Some(Callback::from(move |e: MouseEvent| {
                let target = e.target().unwrap();
                let element = target.dyn_ref::<web_sys::Element>().unwrap();
                let rect = element.get_bounding_client_rect();
                let x = e.client_x() - rect.left() as i32;
                let y = e.client_y() - rect.top() as i32;
                cb.emit((x, y));
            }))
        } else {
            None
        };

        let ontouchstart = if let Some(ref cb) = props.ontouchstart {
            let cb = cb.clone();
            Some(Callback::from(move |e: TouchEvent| {
                e.prevent_default();
                if let Some(touch) = e.touches().get(0) {
                    let target = e.target().unwrap();
                    let element = target.dyn_ref::<web_sys::Element>().unwrap();
                    let rect = element.get_bounding_client_rect();
                    let x = touch.client_x() - rect.left() as i32;
                    let y = touch.client_y() - rect.top() as i32;
                    cb.emit((x, y));
                }
            }))
        } else {
            None
        };

        let src = self.current_blob_url.clone().unwrap_or_else(|| {
            "data:image/gif;base64,R0lGODlhAQABAAAAACH5BAEKAAEALAAAAAABAAEAAAICTAEAOw==".to_string()
        });

        html! {
            <img
                id={props.id.clone()}
                class={props.class.clone()}
                src={src}
                alt="Camera Frame"
                onclick={onclick}
                ontouchstart={ontouchstart}
                style={props.style.clone()}
            />
        }
    }

    fn destroy(&mut self, _ctx: &Context<Self>) {
        // Clean up blob URL
        if let Some(url) = self.current_blob_url.take() {
            let _ = Url::revoke_object_url(&url);
        }
    }
}

/// Calculate reconnect delay with exponential backoff.
fn calculate_reconnect_delay(attempt: u32) -> u32 {
    let base = 500; // 500ms base
    let max = 10000; // 10s max
    let delay = base * 2u32.pow(attempt.min(5));
    delay.min(max)
}

/// Get WebSocket URL based on current page location.
fn get_ws_url(base_url: &Option<String>) -> String {
    if let Some(base) = base_url {
        format!("{base}/ws-stream")
    } else {
        let window = web_sys::window().unwrap();
        let location = window.location();
        let protocol = location.protocol().unwrap_or_else(|_| "http:".to_string());
        let host = location
            .host()
            .unwrap_or_else(|_| "localhost:3000".to_string());

        let ws_protocol = if protocol == "https:" { "wss:" } else { "ws:" };
        format!("{ws_protocol}//{host}/ws-stream")
    }
}

/// Start WebSocket connection from component context.
fn start_ws_connection(ctx: &Context<WsImageStream>) {
    start_ws_connection_with_link(ctx.link());
}

/// Start WebSocket connection with a link (for reconnection from timeout).
fn start_ws_connection_with_link(link: &html::Scope<WsImageStream>) {
    let link = link.clone();
    let base_url = None; // Could get from link props if needed

    spawn_local(async move {
        let url = get_ws_url(&base_url);
        // Connecting to WebSocket at url

        match WebSocket::open(&url) {
            Ok(mut ws) => {
                link.send_message(Msg::Connected);

                // Read messages from the WebSocket stream
                while let Some(msg) = ws.next().await {
                    match msg {
                        Ok(Message::Bytes(data)) => {
                            // Parse binary frame: width(4) + height(4) + frame_num(8) + jpeg
                            if data.len() > 16 {
                                let width =
                                    u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                                let height =
                                    u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
                                let jpeg_data = &data[16..];

                                // Create blob URL for the JPEG data
                                match create_blob_url(jpeg_data) {
                                    Ok(blob_url) => {
                                        link.send_message(Msg::FrameReceived {
                                            blob_url,
                                            width,
                                            height,
                                        });
                                    }
                                    Err(e) => {
                                        link.send_message(Msg::Error(format!(
                                            "Failed to create blob: {e:?}"
                                        )));
                                    }
                                }
                            }
                        }
                        Ok(Message::Text(_)) => {
                            // Ignore text messages
                        }
                        Err(e) => {
                            link.send_message(Msg::Error(format!("WebSocket error: {e:?}")));
                            break;
                        }
                    }
                }

                // Connection closed
                link.send_message(Msg::Disconnected);
            }
            Err(e) => {
                link.send_message(Msg::Error(format!("Failed to connect: {e:?}")));
                link.send_message(Msg::Disconnected);
            }
        }
    });
}

/// Create a blob URL from JPEG data.
fn create_blob_url(jpeg_data: &[u8]) -> Result<String, wasm_bindgen::JsValue> {
    let uint8_array = js_sys::Uint8Array::from(jpeg_data);
    let array = js_sys::Array::new();
    array.push(&uint8_array);

    let options = web_sys::BlobPropertyBag::new();
    options.set_type("image/jpeg");

    let blob = Blob::new_with_u8_array_sequence_and_options(&array, &options)?;
    Url::create_object_url_with_blob(&blob)
}
