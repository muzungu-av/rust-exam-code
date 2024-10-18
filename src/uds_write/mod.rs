use std::io;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;

pub async fn uds_connect(socket_path: &str) -> io::Result<UnixStream> {
    UnixStream::connect(socket_path).await
}

pub async fn uds_write_to(stream: &mut UnixStream, msg: &str) {
    // Отправляем сообщение
    if let Err(e) = stream.write_all(msg.as_bytes()).await {
        eprintln!("Ошибка при отправке: {:?}", e);
    } else {
        // println!("Отправлено сообщение: {}", msg);
    }
}
