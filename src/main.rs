use std::collections::HashMap;
use std::io::ErrorKind;
use std::sync::{Arc};
use json::JsonValue;
use tokio::io;
use tokio::io::{AsyncWriteExt, AsyncBufReadExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use async_recursion::async_recursion;

#[tokio::main]
async fn main() -> io::Result<()> {
	// For some reason, 'localhost' is not the same as '127.0.0.1'.
	// Using 'localhost' gives UB (kinda?).
	let server_addr = "127.0.0.1:5445";
	match TcpListener::bind(server_addr).await {
		Ok(listener) => start_server(listener, server_addr).await,
		Err(error) => match error.kind() {
			ErrorKind::AddrInUse => eprintln!("Address {} is already in use.", server_addr),
			_ => eprintln!("{:?}", error.to_string())
		},
	}

	Ok(())
}

async fn start_server(listener: TcpListener, server_addr: &str) {
	println!("Listening on '{}'...", server_addr);

	let db: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));

	loop {
		let (stream, socket_addr) = listener.accept().await.unwrap();
		println!("User connected: '{}'", socket_addr);

		let mut request = String::new();
		let mut buf: BufReader<TcpStream> = BufReader::new(stream);
		let db = db.clone();

		buf.read_line(&mut request).await.unwrap();
		match json::parse(&request) {
			Ok(data) => {
				print!("Got from user: {}", request);

				update_db(&db, data).await;

				println!("Database Dump: {:#?}", db.lock().await);
				buf.write_all(json::stringify(db.lock().await.clone())
					.as_bytes()).await.unwrap();
			}
			Err(_) => eprintln!("Invalid JSON provided.")
		}
	}
}

#[async_recursion]
async fn update_db(db: &Arc<Mutex<HashMap<String, String>>>, data: JsonValue) {
	match data {
		JsonValue::Object(data) => {
			for (key, value) in data.iter() {
				println!("Adding {}:{} to database...", key, value);
				let mut db = db.lock().await;
				db.insert(key.parse().unwrap(), value.to_string());
				drop(db);
			}
		}
		JsonValue::Array(arr) => {
			for element in arr {
				update_db(db, element).await;
			}
		}
		_ => eprintln!("Invalid JSON provided. Only objects and arrays are accepted."),
	}
}
