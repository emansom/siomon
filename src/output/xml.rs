#![cfg(feature = "xml")]

use crate::model::system::SystemInfo;

pub fn print(info: &SystemInfo) {
    match quick_xml::se::to_string(info) {
        Ok(xml) => {
            println!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
            println!("{xml}");
        }
        Err(e) => eprintln!("XML serialization error: {e}"),
    }
}
