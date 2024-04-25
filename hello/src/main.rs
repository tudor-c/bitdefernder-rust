use std::{
    collections::{HashMap, HashSet}, fs::File, hash::Hash, io::{BufRead, BufReader}, time::Instant
};

use serde::{Deserialize, Serialize};
use serde_json::value::Index;

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

struct IndexedData {
    terms_to_docs: HashMap<Term, HashSet<DocumentId>>,
    idf: HashMap<Term, f64>,
}

impl IndexedData {
    pub fn new() -> Self {
        Self {
            terms_to_docs: HashMap::new(),
            idf: HashMap::new(),
        }
    }
}


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

fn compute_idf(count: u64, n: &HashMap<Term, u64>) -> Result<HashMap<Term, f64>, Box<dyn std::error::Error>> {
    let mut idf = HashMap::new();
    for term in n.keys() {
        let val = (count as f64 - n[term] as f64 + 0.5) / (n[term] as f64 + 0.5) + 1.0;
        idf.insert(term.to_string(), val.ln());
    }
    Ok(idf)
}

fn run_search(data: &IndexType, terms: Vec<&str>) -> HashMap<DocumentId, u64> {
    let mut counter: HashMap<DocumentId, u64> = HashMap::new();

    for term in terms {
        for name in data.get(term).unwrap() {
            let total = counter.entry(name.to_string()).or_insert(0);
            *total += 1;
        }
    }
    counter
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let data_filename = &args[1];
    let data = read_data(data_filename)?;

    let index_timer = Instant::now();
    let index = load_data(&data)?;
    let mut n_val = HashMap::new();
    for (key, _) in &index {
        n_val.insert(key.to_string(), index.get(key).unwrap().len() as u64);
    }

    let indexedData = IndexedData {
        terms_to_docs: index,
        idf: compute_idf(data.len() as u64, &n_val)?
    };

    let mut total = 0;
    for (_, names) in &indexedData.terms_to_docs {
        total += names.len();
    }
    println!("terms: {}\npairs: {}", &indexedData.terms_to_docs.keys().len(), &total);
    println!("indexing took: {:?}", &index_timer.elapsed());

    let search_timer = Instant::now();
    let _result = run_search(&indexedData.terms_to_docs, ["cat.jpg", "DebugProbesKt.bin", "phonenumbers"].to_vec());
    // for (name, count) in result {
    //         println!("name = {}, count = {}", &name, &count);
    // }
    println!("search took: {:?}", &search_timer.elapsed());
    Ok(())
}
