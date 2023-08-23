
use std::fs;
use std::io;
use std::usize;

struct ColumnHeaders {
    data: Vec<String>,
}

struct Blacklist {
    data: Vec<String>,
}

struct LogData {
    data: Vec<f32>,
}

fn main() {
    let log = fs::read_to_string("./data/log1.csv")
        .expect("Should be able to read log file");
    let headerindex = &log.find("\r\n");
    let headers = match headerindex {
        Some(headerindex) => &log[0..*headerindex],
        None => "Not found",
    };
    print!("Log headers: {}",&headers)
}
