use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufRead, BufReader}, time::Instant,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct FileData {
    /// name of the zip archive
    name: String,
    /// list of files in the zip archive
    files: Vec<String>,
}

type Term = String;
type DocumentId = String;
type IndexType = HashMap<Term, HashSet<DocumentId>>;

fn read_data(data_filename: &str) -> Result<Vec<FileData>, Box<dyn std::error::Error>> {
    let file = File::open(data_filename)?;
    let reader = BufReader::new(file);

    let mut data: Vec<FileData> = Vec::new();
    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        data.push(serde_json::from_str(line)?);
    }
    Ok(data)
}

fn load_data(data: &Vec<FileData>) -> Result<IndexType, Box<dyn std::error::Error>> {
    let mut index = IndexType::new();

    for filedata in data {
        let name = &filedata.name;
        let files = &filedata.files;
        for item in files {
            let tokens = item.split('/');
            for token in tokens {
                index.entry(token.to_string())
                    .or_insert(HashSet::new())
                    .insert(name.to_string());
            }
        }
    }
    Ok(index)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let data_filename = &args[1];
    let data = read_data(data_filename)?;

    let time = Instant::now();
    let index = load_data(&data)?;

    let mut total = 0;
    for (_, names) in &index {
        total += names.len();
    }
    println!("terms: {}\npairs: {}", &index.keys().len(), &total);
    println!("elapsed: {:?}", &time.elapsed());
    Ok(())
}
