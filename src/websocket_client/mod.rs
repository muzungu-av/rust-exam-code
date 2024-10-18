use async_trait::async_trait;
// use chrono::{Local, Utc};
// use chrono::Utc;
use ezsockets::{ClientConfig, CloseCode, CloseFrame, Error};
use std::sync::{Arc, Mutex};
use tracing::debug;
use url::Url;

use crate::queue::OUTCOMING_QUEUE;

pub enum Call {
    NewLine(String),
}

pub struct WebSocketClient {
    handle: Mutex<ezsockets::Client<WebSocketClient>>,
    pub initialized: bool, // Поле для проверки инициализации
}

#[async_trait]
impl ezsockets::ClientExt for WebSocketClient {
    type Call = Call;

    async fn on_text(&mut self, text: String) -> Result<(), Error> {
        // let current_time = Local::now();
        // let formatted_time = current_time.format("%H:%M:%S%.6f");
        // println!("{} ", formatted_time);
        OUTCOMING_QUEUE.push(text); //в очередь на чтение
        Ok(())
    }

    async fn on_binary(&mut self, bytes: Vec<u8>) -> Result<(), Error> {
        tracing::info!("received bytes: {:?}", bytes);
        Ok(())
    }

    async fn on_call(&mut self, call: Self::Call) -> Result<(), Error> {
        match call {
            Call::NewLine(line) => {
                if line == "exit" {
                    tracing::info!("exiting...");
                    self.handle
                        .lock()
                        .unwrap()
                        .close(Some(CloseFrame {
                            code: CloseCode::Normal,
                            reason: "adios!".to_string(),
                        }))
                        .unwrap();
                    return Ok(());
                }
                debug!("sending {}", line);
                self.handle.lock().unwrap().text(line).unwrap();
            }
        }
        Ok(())
    }
}

impl WebSocketClient {
    pub async fn new(url: &str) -> Arc<Self> {
        let url = Url::parse(url).unwrap();
        let config = ClientConfig::new(url);
        let (handle, future) = ezsockets::connect(
            |handle| WebSocketClient {
                handle: Mutex::new(handle),
                initialized: true,
            },
            config,
        )
        .await;
        let client = Arc::new(WebSocketClient {
            handle: Mutex::new(handle),
            initialized: true,
        });
        tokio::spawn(future);
        client
    }

    pub fn send_message(&self, message: &str) {
        self.handle
            .lock()
            .unwrap()
            .call(Call::NewLine(message.to_string()))
            .unwrap();
    }

    pub fn close(&self) -> Result<bool, String> {
        match self.handle.lock() {
            Ok(handle) => {
                if handle
                    .close(Some(CloseFrame {
                        code: CloseCode::Normal,
                        reason: "Goodbye!".to_string(),
                    }))
                    .is_ok()
                {
                    Ok(true)
                } else {
                    Err("Failed to close the connection".to_string())
                }
            }
            Err(_) => Err("Failed to lock the handle".to_string()),
        }
    }
}
