use std::{
    fs::File,
    io::{prelude::*, BufReader}
};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct FileData {
    name: String,
    filenames: Vec<String>
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let json_file = &args[1];

    let file = File::open(&json_file)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let value: FileData = serde_json::from_str(&line?)?;
        println!("{} has {} files", value.name, value.filenames.len());
    }

    Ok(())
}
