
use hello::{SearchData, SearchResult};

pub fn main() {
    println!("client smecher de rust!\n");

    let mut args: Vec<String> = std::env::args().collect();
    args.remove(0);

    let search_data = SearchData {
        terms: args,
        min_score: Some(0.0),
        max_length: Some(1000),
    };
    let body = serde_json::to_string(&search_data).unwrap();
    let client = reqwest::blocking::Client::new();
    let res = client.post("http://127.0.0.1:8000/search")
        .body(body)
        .send()
        .expect("request result error")
        .text()
        .unwrap();

    let result = serde_json::from_str::<SearchResult>(&res.to_string()).unwrap() as SearchResult;
    for entry in result.matches {
        println!("{}", entry.md5);
    }
}