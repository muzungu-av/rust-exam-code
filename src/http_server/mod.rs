/*
HTTP-сервер. С интегрированными WebSocketClient и TwoWayQueue.
http-прослушивает внешние команды (send_message, stop) и управляет WS

http.send_message поставит сообщение в очередь TwoWayQueue
откуда сразу будет прочитано (pop) и отправлено в WS.

Хотя такая возможность тут реализована, использовать только для каких-то
не ключевых команд, для которых не имеет значение скорость исполнения.

Ключевые команды обмена, будут использовать ту же очередь, но в другом месте...
*/
use crate::queue::TwoWayQueue;
use crate::websocket_client::WebSocketClient;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{debug, error};
use warp::http::StatusCode;
use warp::Filter;

pub struct HttpServer {
    server_handle: tokio::task::JoinHandle<()>,
    queue_handle: std::thread::JoinHandle<()>,
}

pub struct HttpServerConfig {
    pub port: u16,
    pub client: Arc<WebSocketClient>,
    pub incoming_queue: Arc<TwoWayQueue>,
}

impl HttpServer {
    pub async fn start(config: HttpServerConfig) -> Self {
        println!("Создаем сервер HTTP");

        let client = config.client.clone();
        let queue = config.incoming_queue.clone();

        // запрос send_message
        let send_message_filter = warp::path("send_message")
            .and(warp::post())
            .and(with_queue(queue.clone()))
            .and(warp::body::content_length_limit(1024 * 1))
            .and(warp::body::bytes())
            .and_then(send_message);
        //запрос stop
        let stop_filter = warp::path("stop")
            .and(warp::post())
            .and(with_client(client.clone()))
            .and_then(stop_websocket);

        let routes = send_message_filter.or(stop_filter);
        let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
        let server = warp::serve(routes).run(addr);
        let server_handle = tokio::spawn(server);

        let client_clone = client.clone();
        let queue_handle = std::thread::spawn(move || {
            while let Some(value) = queue.pop() {
                client_clone.send_message(&value); //отправка в WS
            }
        });

        HttpServer {
            server_handle,
            queue_handle,
        }
    }

    pub async fn await_completion(self) {
        self.server_handle.await.expect("HTTP server crashed");
        self.queue_handle.join().expect("Queue thread crashed");
    }
}

fn with_client(
    client: Arc<WebSocketClient>,
) -> impl Filter<Extract = (Arc<WebSocketClient>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || client.clone())
}

fn with_queue(
    queue: Arc<TwoWayQueue>,
) -> impl Filter<Extract = (Arc<TwoWayQueue>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || queue.clone())
}

pub async fn send_message(
    queue: Arc<TwoWayQueue>,
    body: bytes::Bytes,
) -> Result<impl warp::Reply, warp::Rejection> {
    match std::str::from_utf8(&body) {
        Ok(message) => {
            // debug!("(http) Received message: {}", message);
            queue.push(message.to_string());
            Ok(warp::reply::with_status(
                "Message received",
                warp::http::StatusCode::OK,
            ))
        }
        Err(_) => Ok(warp::reply::with_status(
            "Invalid message",
            warp::http::StatusCode::BAD_REQUEST,
        )),
    }
}

async fn stop_websocket(client: Arc<WebSocketClient>) -> Result<impl warp::Reply, warp::Rejection> {
    debug!("Stop WebSocket.........");
    match client.close() {
        Ok(true) => Ok(StatusCode::OK),
        Ok(false) => Ok(StatusCode::INTERNAL_SERVER_ERROR),
        Err(e) => {
            error!("Error closing websocket: {}", e);
            Ok(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
