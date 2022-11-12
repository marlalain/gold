use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time;
use time::SystemTime;

use async_recursion::async_recursion;
use bytes::BytesMut;
use json::object::Object;
use json::JsonValue;
use tokio::io::Result;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::spawn;
use tokio::sync::Mutex;

use crate::http::HttpMethods;

mod http;

#[tokio::main]
async fn main() -> Result<()> {
    // For some reason, 'localhost' is not the same as '127.0.0.1'.
    // Using 'localhost' doesn't work.
    let server_addr = "0.0.0.0:5445";
    match TcpListener::bind(server_addr).await {
        Ok(listener) => start_server(listener, server_addr).await?,
        Err(error) => match error.kind() {
            ErrorKind::AddrInUse => eprintln!("Address {} is already in use.", server_addr),
            _ => eprintln!("{:?}", error.to_string()),
        },
    }

    Ok(())
}

async fn start_server(listener: TcpListener, server_addr: &str) -> Result<()> {
    println!("Listening on '{}'...", server_addr);

    let db: Arc<Mutex<HashMap<String, Object>>> = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (stream, socket_addr) = listener.accept().await.unwrap();
        let now = SystemTime::now();
        println!("[{}]: User connected", socket_addr);
        let db = db.clone();

        spawn(async move {
            let mut buf: BufReader<TcpStream> = BufReader::new(stream);
            let mut result = String::new();
            let mut content_length = 0usize;
            let mut method = HttpMethods::default();
            let mut key: String = String::new();
            'outer: loop {
                match buf.read_line(&mut result).await {
                    Ok(0) => break 'outer,
                    Ok(_) => {
                        if result.eq("\r\n") {
                            break 'outer;
                        }

                        if result.starts_with("Content-Length") {
                            content_length = usize::from_str(
                                &*result.replace("Content-Length: ", "").replace("\r\n", ""),
                            )
                            .unwrap();
                        } else if result.contains("HTTP/") {
                            let first_line = result.split_whitespace().collect::<Vec<&str>>();
                            key = first_line
                                .get(1)
                                .unwrap()
                                .parse::<String>()
                                .unwrap()
                                .replace("/", "");
                            method = HttpMethods::from(first_line.get(0).unwrap().to_string());
                        }

                        print!("[{}]: {}", socket_addr, result);
                        result.clear();
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                        buf.write_all(b"HTTP/1.1 500 Internal Server Error")
                            .await
                            .unwrap();
                        break 'outer;
                    }
                }
            }

            match method {
                HttpMethods::GET => match query_db(&db, key).await {
                    None => {
                        eprintln!("[{}]: Invalid JSON", socket_addr);
                        buf.write_all(b"HTTP/1.1 404 Not Found").await.unwrap();
                    }
                    Some(entry) => {
                        let body = json::stringify(entry.clone());
                        let response = format!("HTTP/1.1 200 OK\r\n\r\n{}", body);
                        buf.write_all(response.as_bytes()).await.unwrap();
                    }
                },
                HttpMethods::POST => {
                    let mut buffer = BytesMut::with_capacity(content_length);
                    buf.read_buf(&mut buffer).await.unwrap();
                    let raw_body = String::from_utf8(Vec::from(buffer)).unwrap();

                    if let Ok(body) = json::parse(&*raw_body) {
                        update_db(&db, body.clone(), key, socket_addr).await;
                        println!("[{}]: {:?}", socket_addr, body);
                        buf.write_all(b"HTTP/1.1 202 Accepted").await.unwrap();
                    } else {
                        eprintln!("[{}]: Invalid JSON", socket_addr);
                        buf.write_all(b"HTTP/1.1 400 Bad Request").await.unwrap();
                    }
                }
                HttpMethods::DELETE => {
                    let mut _db = db.lock().await;
                    _db.remove(&*key);
                    println!("[{}]: Deleting resource", socket_addr);
                    buf.write_all(b"HTTP/1.1 202 Accepted").await.unwrap();
                }
            }

            if let Ok(elapsed) = now.elapsed() {
                println!("[{}]: Finished in {}μs", socket_addr, elapsed.as_micros());
            }
        });
    }
}

#[async_recursion]
async fn update_db(
    db: &Arc<Mutex<HashMap<String, Object>>>,
    data: JsonValue,
    key: String,
    socket_addr: SocketAddr,
) {
    match data {
        JsonValue::Object(data) => {
            let mut db = db.lock().await;
            db.insert(key, data);
        }
        JsonValue::Array(arr) => {
            for element in arr {
                update_db(db, element, key.clone() + "-ex", socket_addr).await;
            }
        }
        _ => eprintln!(
            "[{}]: Invalid JSON provided. Only objects and arrays are accepted.",
            socket_addr
        ),
    }
}

async fn query_db(db: &Arc<Mutex<HashMap<String, Object>>>, key: String) -> Option<Object> {
    db.lock().await.get(&key).cloned()
}
