#[macro_use]
extern crate rocket;

use std::{
    collections::HashMap, fs::File, io::{BufRead, BufReader, Read, Seek, SeekFrom, Write}, path::Path, sync::{Arc, RwLock}, time::Instant
};

use rocket::{fs::{FileName, FileServer, TempFile}, serde::json::Json, tokio::fs, State};

use serde::{Deserialize, Serialize};
use rmp_serde::{Deserializer, Serializer};

type Term = String;
type DocumentId = String;

#[derive(Default, Serialize, Deserialize)]
struct IndexedData {
    terms_to_docs_idx: HashMap<Term, Vec<usize>>,
    names: Vec<String>,
    idf: HashMap<Term, f64>,
    num_docs: usize,
}

impl IndexedData {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Serialize)]
struct Greeting {
    message: String,
}

#[derive(Deserialize)]
struct SearchData {
    terms: Vec<String>,
    min_score: Option<f64>,
    max_length: Option<u32>,
}

#[derive(Serialize)]
struct SearchResult {
    total: usize,
    matches: Vec<SearchMatch>
}

#[derive(Serialize)]
struct SearchMatch {
    md5: DocumentId,
    score: f64
}

#[derive(Debug, Serialize, Deserialize)]
struct FileData {
    /// name of the zip archive
    name: DocumentId,
    /// list of files in the zip archive
    files: Vec<String>,
}

fn compute_idf(terms_to_docs: &HashMap<Term, Vec<usize>>) -> HashMap<Term, f64> {
    let n = terms_to_docs.len() as f64;
    let mut terms_idf = HashMap::new();
    for (term, docs) in terms_to_docs {
        let nq = docs.len() as f64;
        let idf = ((n - nq + 0.5) / (nq + 0.5)).ln();
        terms_idf.insert(term.clone(), idf);
    }

    terms_idf
}

fn load_data(data_filename: &str, limit: Option<usize>) -> eyre::Result<IndexedData> {
    let file = File::open(data_filename)?;
    let reader = BufReader::new(file);

    let mut index = IndexedData::new();
    let mut names: Vec<DocumentId> = Vec::new();

    for (count, line) in reader.lines().take(limit.unwrap_or(usize::MAX)).enumerate() {
        let line = line?;
        let fd: FileData = serde_json::from_str(&line)?;
        let name = fd.name.clone();

        for file in fd.files {
            for term in file.split('/') {
                if let Some(set) = index.terms_to_docs_idx.get_mut(term) {
                    if set.last() != Some(&count) {
                        set.push(count);
                    }
                } else {
                    let set: Vec<usize> = vec![count];
                    index.terms_to_docs_idx.insert(term.to_string(), set);
                }
            }
        }

        names.push(name);
        index.num_docs += 1;
    }

    index.idf = compute_idf(&index.terms_to_docs_idx);
    index.names = names;
    Ok(index)
}

fn run_search(data: &IndexedData, terms: Vec<&str>, min_score: f64, _max_length: u32) -> SearchResult {
    let mut counter: HashMap<DocumentId, u64> = HashMap::new();
    for term in &terms {
        if let Some(docs) = data.terms_to_docs_idx.get(*term) {
            for doc in docs {
                let x = counter.entry(data.names[*doc].to_string()).or_insert(0);
                *x += 1;
            }
        }
    }

    let mut matches: Vec<SearchMatch> = Vec::new();
    for (doc, cnt) in counter {
        let res = SearchMatch {
            md5: doc.to_string(),
            score: cnt as f64 / terms.len() as f64
        };
        if res.score >= min_score {
            matches.push(res);
        }
    }
    matches.sort_by(|a, b| b.score.total_cmp(&a.score));
    SearchResult {
        total: matches.len(),
        matches,
    }
}

#[get("/")]
fn index() -> Json<Greeting> {
    Json(Greeting {
        message: "Hello, welcome to our server!".to_string(),
    })
}

#[derive(FromForm)]
struct Upload<'r> {
    file: TempFile<'r>,
}

#[post("/search_by_file", data = "<upload>")]
fn saerch_by_file(upload: rocket::form::Form<Upload<'_>>, server_state: &State<Arc<RwLock<ServerState>>>) {
    let file = File::open(upload.file.path().unwrap()).unwrap();
    let reader = BufReader::new(file);
    let mut zip = zip::ZipArchive::new(reader).unwrap();
    let mut filenames = Vec::new();

    for i in 0..zip.len() {
        let f = zip.by_index(i).unwrap();
        // let mut tokens: Vec<_> = f.name().split('/').map(|x| x.to_string()).collect();
        // filenames.append(&mut tokens);
        filenames.push(f.name().to_string());
    }

    let mut tokens = Vec::new();
    for filename in &filenames {
        tokens.extend(filename.split('/'));
    }

    let index = &server_state.read().map_err(|err| format!("Err: {:#}", err)).unwrap().index;
    let result = run_search(index, tokens, 0., 10000);
    for match_str in &result.matches {
        println!("{}", match_str.md5);
    }
}

#[post("/search", data = "<req>")]
fn search(req: Json<SearchData>, server_state: &State<Arc<RwLock<ServerState>>>) -> Result<Json<SearchResult>, String> {
    let index = &server_state.read().map_err(|err| format!("Err: {:#}", err))?.index;
    let terms = req.terms.iter().map(|s| s.as_str()).collect();
    let result = run_search(index, terms, req.min_score.unwrap(), req.max_length.unwrap());
    Ok(Json(result))
}

#[derive(Default)]
struct ServerState {
    index: IndexedData,
}

#[rocket::main]
async fn main() -> eyre::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let data_filename = &args[1];
    let limit = args
        .get(2)
        .map(|x| x.parse::<usize>())
        .transpose()?;

    println!("loading {data_filename}...");
    let start = Instant::now();

    let serialized_data_path = "data/deserialized_index";
    let mut data: IndexedData;
    if Path::new(&serialized_data_path).exists() {
        println!("reading serialized data instead...");
        let mut file = File::open(&serialized_data_path)?;
        // let size = file.metadata().unwrap().len();
        // let mut buffer: Vec<u8> = vec![0; size as usize];
        // let n = file.read(&mut buffer[..])?;
        // println!("Read {} bytes from {}", n, serialized_data_path);
        data = rmp_serde::from_read(file)?;
    }
    else {
        data = load_data(data_filename, limit)?;
    }

    let pair_count = data
        .terms_to_docs_idx.values().map(|docs| docs.len())
        .sum::<usize>();
    println!(
        "loaded data for {} docs, {} terms, {} term-docid pairs, in {:.2}s",
        data.num_docs,
        data.terms_to_docs_idx.len(),
        pair_count,
        start.elapsed().as_secs_f64(),
    );

    let mut buffer = Vec::new();
    data.serialize(&mut Serializer::new(&mut buffer)).unwrap();

    let mut file =  std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&serialized_data_path)?;
    file.write_all(&buffer)?;

    let server_state = Arc::new(RwLock::new(ServerState { index: data }));
    rocket::build()
        .manage(server_state)
        .mount("/", routes![index, search, saerch_by_file])
        .mount("/dashboard", FileServer::from("static"))
        .ignite()
        .await?
        .launch()
        .await?;

    Ok(())
}
