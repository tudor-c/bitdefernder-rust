#[macro_use]
extern crate rocket;

use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    sync::{Arc, RwLock},
    time::Instant,
};

use rocket::{fs::FileServer, serde::json::Json, State};

use serde::{Deserialize, Serialize};

type Term = String;
type DocumentId = String;

#[derive(Default)]
struct IndexedData {
    terms_to_docs: HashMap<Term, Vec<DocumentId>>,
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
    min_score: Option<i32>,
    max_length: Option<f64>,
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

fn compute_idf(terms_to_docs: &HashMap<Term, Vec<DocumentId>>) -> HashMap<Term, f64> {
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
    for line in reader.lines().take(limit.unwrap_or(usize::MAX)) {
        let line = line?;

        let fd: FileData = serde_json::from_str(&line)?;
        for file in fd.files {
            for term in file.split("/") {
                if let Some(set) = index.terms_to_docs.get_mut(term) {
                    if set.last() != Some(&fd.name) {
                        set.push(fd.name.clone());
                    }
                } else {
                    let mut set = Vec::new();
                    set.push(fd.name.clone());
                    index.terms_to_docs.insert(term.to_string(), set);
                }
            }
        }

        index.num_docs += 1;
    }

    index.idf = compute_idf(&index.terms_to_docs);
    Ok(index)
}

fn run_search(data: &IndexedData, terms: Vec<&str>) -> SearchResult {
    let mut counter: HashMap<DocumentId, u64> = HashMap::new();
    for term in &terms {
        if let Some(docs) = data.terms_to_docs.get(*term) {
            for doc in docs {
                let x = counter.entry(doc.to_string()).or_insert(0);
                *x += 1;
            }
        }
    }

    let mut matches: Vec<SearchMatch> = Vec::new();
    for (doc, cnt) in counter {
        matches.push(SearchMatch {
                md5: doc.to_string(),
                score: cnt as f64 / terms.len() as f64
            }
        );
    }
    matches.sort_by(|a, b| b.score.total_cmp(&a.score));
    SearchResult {
        total: matches.len(),
        matches: matches,
    }
}

#[get("/")]
fn index() -> Json<Greeting> {
    Json(Greeting {
        message: "Hello, welcome to our server!".to_string(),
    })
}

#[post("/search", data = "<req>")]
fn search(req: Json<SearchData>, server_state: &State<Arc<RwLock<ServerState>>>) -> Result<Json<SearchResult>, String> {
    let index = &server_state.read().map_err(|err| format!("Err: {:#}", err))?.index;
    let terms = req.terms.iter().map(|s| s.as_str()).collect();
    let result = run_search(&index, terms);
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
        .map(|x| usize::from_str_radix(&x, 10))
        .transpose()?;

    println!("loading {data_filename}...");
    let start = Instant::now();

    let data = load_data(data_filename, limit)?;

    let pair_count = data
        .terms_to_docs
        .iter()
        .map(|(_, docs)| docs.len())
        .sum::<usize>();
    println!(
        "loaded data for {} docs, {} terms, {} term-docid pairs, in {:.2}s",
        data.num_docs,
        data.terms_to_docs.len(),
        pair_count,
        start.elapsed().as_secs_f64(),
    );

    let start = Instant::now();
    let search = vec!["lombok", "AUTHORS", "README.md"];
    let matches = run_search(&data, search);
    println!(
        "search found {} matches in {:.2}s",
        matches.matches.len(),
        start.elapsed().as_secs_f64(),
    );

    let server_state = Arc::new(RwLock::new(ServerState { index: data }));
    rocket::build()
        .manage(server_state)
        .mount("/", routes![index, search])
        .mount("/dashboard", FileServer::from("static"))
        .ignite()
        .await?
        .launch()
        .await?;

    Ok(())
}
