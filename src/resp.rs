use std::borrow::Borrow;

use crate::{query_db, update_db, Database};

pub enum RespCommand {
    PING,
    GET,
    SET,
    EXISTS,
}

impl RespCommand {
    pub fn by_str(string: &String) -> Self {
        let command = string.split(" ").collect::<Vec<&str>>()[0];
        match command {
            "PING" => Self::PING,
            "GET" => Self::GET,
            "SET" => Self::SET,
            "EXISTS" => Self::EXISTS,
            _ => panic!("not a valid resp command"),
        }
    }

    pub fn non_db(&self) -> bool {
        match self {
            RespCommand::PING => true,
            _ => false,
        }
    }

    pub async fn process(&self, db: &Database, string: String) -> Vec<u8> {
        match self {
            RespCommand::GET => {
                let words = string.split_whitespace().collect::<Vec<&str>>();

                match query_db(&db, words[1].clone().to_string()).await {
                    None => {
                        eprintln!("could not find value from the key");
                        "-ERROR NOT FOUND\r\n".to_string()
                    }
                    Some(entry) => {
                        let res: String = json::stringify(entry.clone());
                        format!("+{}\r\n", res)
                    }
                }
            }
            RespCommand::SET => {
                let slices = string.splitn(3, " ").collect::<Vec<&str>>();
                let slices = slices.as_slice();

                if let Ok(body) = json::parse(&slices[2]) {
                    update_db(&db, body.clone(), (&slices[1]).clone().to_string(), None).await;

                    println!("added key {}", &slices[1]);
                    "+OK\r\n".to_string()
                } else {
                    eprintln!("invalid json");
                    "-ERROR INVALID JSON\r\n".to_string()
                }
            }
            RespCommand::EXISTS => {
                let words = string.split_whitespace().collect::<Vec<&str>>();

                match query_db(&db, words[1].clone().to_string()).await {
                    None => ":0\r\n".to_string(),
                    Some(_) => ":1\r\n".to_string(),
                }
            }
            other => other.process_non_db().to_string(),
        }
        .as_bytes()
        .to_vec()
    }

    pub fn process_non_db(&self) -> &'static str {
        return match self {
            RespCommand::PING => "+PONG\r\n",
            _ => "-ERROR IMPOSSIBLE CASE\r\n",
        };
    }
}
