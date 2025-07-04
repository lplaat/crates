/*
 * Copyright (c) 2023-2025 Bastiaan van der Plaat
 *
 * SPDX-License-Identifier: MIT
 */

#![doc = include_str!("../README.md")]
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#![forbid(unsafe_code)]

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use rust_embed::Embed;
use small_websocket::Message;
use tiny_webview::{Event, EventLoopBuilder, LogicalSize, WebviewBuilder};

use crate::ipc::{IPC_CONNECTIONS, IpcConnection, ipc_message_handler};

mod config;
mod dmx;
mod ipc;
mod usb;

#[derive(Embed)]
#[folder = "$OUT_DIR/web"]
struct WebAssets;

// MARK: Main
fn main() {
    let mut event_loop = EventLoopBuilder::build();

    let webview = Arc::new(Mutex::new(
        WebviewBuilder::new()
            .title("BassieLight")
            .size(LogicalSize::new(1024.0, 768.0))
            .min_size(LogicalSize::new(640.0, 480.0))
            .center()
            .remember_window_state(true)
            .force_dark_mode(true)
            .load_rust_embed::<WebAssets>()
            .internal_http_serve_handle(|req| {
                if req.url.path() == "/ipc" {
                    return Some(small_websocket::upgrade(req, |mut ws| {
                        IPC_CONNECTIONS
                            .lock()
                            .expect("Failed to lock IPC connections")
                            .push(IpcConnection::WebSocket(ws.clone()));
                        loop {
                            let message = match ws.recv_non_blocking() {
                                Ok(message) => message,
                                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                                    continue;
                                }
                                Err(err) => panic!("WebSocket recv error: {}", err),
                            };
                            match message {
                                Some(Message::Text(text)) => {
                                    ipc_message_handler(
                                        IpcConnection::WebSocket(ws.clone()),
                                        &text,
                                    );
                                }
                                Some(Message::Close(_, _)) => break,
                                None => {
                                    thread::sleep(Duration::from_millis(100));
                                }
                                _ => {}
                            }
                        }
                        IPC_CONNECTIONS
                            .lock()
                            .expect("Failed to lock IPC connections")
                            .retain(|conn| conn != &IpcConnection::WebSocket(ws.clone()));
                    }));
                }
                None
            })
            .build(),
    ));

    let config = config::load_config("config.json").expect("Can't load config.json");
    event_loop.run(move |event| match event {
        Event::PageLoadFinished => {
            IPC_CONNECTIONS
                .lock()
                .expect("Failed to lock IPC connections")
                .push(IpcConnection::WebviewIpc(webview.clone()));

            let config = config.clone();
            thread::spawn(move || {
                if let Some(device) = usb::find_udmx_device() {
                    dmx::dmx_thread(device, config);
                } else {
                    eprintln!("[RUST] No uDMX device found");
                }
            });
        }
        Event::PageMessageReceived(message) => {
            ipc_message_handler(IpcConnection::WebviewIpc(webview.clone()), &message)
        }

        _ => {}
    });
}
