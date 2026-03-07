#![cfg(feature = "json")]

use crate::model::system::SystemInfo;

pub fn print(info: &SystemInfo) {
    match serde_json::to_string_pretty(info) {
        Ok(json) => println!("{json}"),
        Err(e) => eprintln!("JSON serialization error: {e}"),
    }
}
