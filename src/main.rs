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
use json::{object, JsonValue};
use tokio::io::Result;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::spawn;
use tokio::sync::Mutex;

use crate::http::HttpMethods;
use crate::server::ServerMode;

mod http;
mod server;

pub type Database = Arc<Mutex<HashMap<String, Object>>>;

#[tokio::main]
async fn main() -> Result<()> {
    // For some reason, 'localhost' is not the same as '127.0.0.1'.
    // Using 'localhost' doesn't work.
    let server_addr = "0.0.0.0:5445";
    match TcpListener::bind(server_addr).await {
        Ok(listener) => {
            println!("Listening on '{}'...", server_addr);
            let db: Database = Arc::new(Mutex::new(HashMap::new()));
            let mode = ServerMode::RESP;
            mode.run(listener, db).await;
        }
        Err(error) => match error.kind() {
            ErrorKind::AddrInUse => eprintln!("Address {} is already in use.", server_addr),
            _ => eprintln!("{:?}", error.to_string()),
        },
    }

    Ok(())
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
