use std::str::FromStr;
use std::time::SystemTime;

use bytes::BytesMut;
use tokio::io::Result;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::spawn;

use crate::http::HttpMethods;
use crate::resp::RespCommand;
use crate::{query_db, update_db, Database};

#[derive(Default)]
pub enum ServerMode {
    #[default]
    HTTP,
    RESP,
}

impl ServerMode {
    pub async fn run(&self, listener: TcpListener, db: Database) -> Result<()> {
        match self {
            Self::HTTP => Self::start_http_server(listener, db).await,
            Self::RESP => Self::start_resp_server(listener, db).await,
        }
    }

    async fn start_http_server(listener: TcpListener, db: Database) -> Result<()> {
        println!("gold created as an HTTP server...");

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
                                    .get(0)
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
                            buf.write_all(b"HTTP/0.1 500 Internal Server Error")
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
                            buf.write_all(b"HTTP/0.1 404 Not Found").await.unwrap();
                        }
                        Some(entry) => {
                            let body = json::stringify(entry.clone());
                            let response = format!("HTTP/0.1 200 OK\r\n\r\n{}", body);
                            buf.write_all(response.as_bytes()).await.unwrap();
                        }
                    },
                    HttpMethods::POST => {
                        let mut buffer = BytesMut::with_capacity(content_length);
                        buf.read_buf(&mut buffer).await.unwrap();
                        let raw_body = String::from_utf8(Vec::from(buffer)).unwrap();

                        if let Ok(body) = json::parse(&*raw_body) {
                            update_db(&db, body.clone(), key, Some(socket_addr)).await;
                            println!("[{}]: {:?}", socket_addr, body);
                            buf.write_all(b"HTTP/0.1 202 Accepted").await.unwrap();
                        } else {
                            eprintln!("[{}]: Invalid JSON", socket_addr);
                            buf.write_all(b"HTTP/0.1 400 Bad Request").await.unwrap();
                        }
                    }
                    HttpMethods::DELETE => {
                        let mut _db = db.lock().await;
                        _db.remove(&*key);
                        println!("[{}]: Deleting resource", socket_addr);
                        buf.write_all(b"HTTP/0.1 202 Accepted").await.unwrap();
                    }
                }

                if let Ok(elapsed) = now.elapsed() {
                    println!("[{}]: Finished in {}μs", socket_addr, elapsed.as_micros());
                }
            });
        }
    }

    async fn start_resp_server(listener: TcpListener, db: Database) -> Result<()> {
        println!("gold is accepting RESP calls...");

        loop {
            let (stream, socket_addr) = listener.accept().await.unwrap();
            let db = db.clone();

            spawn(async move {
                let mut buf = BufReader::new(stream);
                let mut request = String::new();

                loop {
                    match buf.read_line(&mut request).await {
                        Ok(0) => break,
                        Ok(_) => {
                            for raw_line in request.clone().split("\r\n") {
                                let line = raw_line.replace("\r\n", "");
                                if line == "" {
                                    continue;
                                }

                                let now = SystemTime::now();
                                let command = RespCommand::by_str(&line);

                                buf.write(&*command.process(&db, line).await).await.unwrap();
                                (&mut request).clear();

                                if let Ok(elapsed) = now.elapsed() {
                                    println!(
                                        "[{}]: finished in {}μs",
                                        socket_addr,
                                        elapsed.as_micros()
                                    );
                                }
                            }
                        }
                        Err(err) => eprintln!("{:?}", err),
                    }
                }
            });
        }
    }
}
