extern crate core;

use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::sync::Arc;

use async_recursion::async_recursion;
use json::object::Object;
use json::JsonValue;
use tokio::io::Result;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

use crate::server::ServerMode;

mod http;
mod resp;
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
            mode.run(listener, db).await?;
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
    socket_addr: Option<SocketAddr>,
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
        _ => {
            if socket_addr.is_none() {
                eprintln!("invalid JSON provided. only objects and arrays are accepted");
                return;
            }

            eprintln!(
                "[{}]: Invalid JSON provided. Only objects and arrays are accepted.",
                socket_addr.unwrap()
            )
        }
    }
}

async fn query_db(db: &Arc<Mutex<HashMap<String, Object>>>, key: String) -> Option<Object> {
    db.lock().await.get(&key).cloned()
}
