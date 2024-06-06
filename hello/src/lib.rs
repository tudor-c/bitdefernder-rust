use serde::{Deserialize, Serialize};

pub type DocumentId = String;

#[derive(Serialize, Deserialize)]
pub struct SearchData {
    pub terms: Vec<String>,
    pub min_score: Option<f64>,
    pub max_length: Option<u32>,
}


#[derive(Serialize, Deserialize)]
pub struct SearchMatch {
    pub md5: DocumentId,
    pub score: f64
}

#[derive(Serialize, Deserialize)]
pub struct SearchResult {
    pub total: usize,
    pub matches: Vec<SearchMatch>
}