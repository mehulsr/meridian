//! Inverted index for token search on string columns.
//!
//! Port of `meridian.index.inverted`. Tokenizes strings and builds a postings
//! dict mapping token → row_ids. Supports single and AND multi-term search.

use std::collections::HashMap;

use regex::Regex;

use crate::core::types::Value;
use crate::storage::column::Column;

/// Lazy-initialized regex for token splitting.
fn token_regex() -> &'static Regex {
    // [a-z0-9]+ matches lowercase alphanumeric runs
    Box::leak(Box::new(Regex::new(r"[a-z0-9]+").unwrap()))
}

/// Port of the Python `InvertedIndex` class.
#[derive(Debug, Clone)]
pub struct InvertedIndex {
    pub column_name: String,
    postings: HashMap<String, Vec<usize>>,
}

impl InvertedIndex {
    /// Python `InvertedIndex(column_name)`.
    pub fn new(column_name: impl Into<String>) -> InvertedIndex {
        InvertedIndex { column_name: column_name.into(), postings: HashMap::new() }
    }

    /// Python `InvertedIndex.build` — tokenize all strings, populate postings.
    pub fn build(&mut self, column: &Column) {
        self.postings.clear();
        let regex = token_regex();
        for (row_id, value) in column.to_list().into_iter().enumerate() {
            if value.is_null() {
                continue;
            }
            let text = value.as_str().unwrap_or("").to_lowercase();
            for token in regex.find_iter(&text) {
                let tok = token.as_str().to_string();
                let postings = self.postings.entry(tok).or_insert_with(Vec::new);
                if postings.last() != Some(&row_id) {
                    postings.push(row_id);
                }
            }
        }
    }

    /// Python `InvertedIndex.search` — lookup single term.
    pub fn search(&self, term: &str) -> Vec<usize> {
        self.postings
            .get(&term.to_lowercase())
            .cloned()
            .unwrap_or_default()
    }

    /// Python `InvertedIndex.search_and` — intersection of multiple terms.
    pub fn search_and(&self, terms: &[&str]) -> Vec<usize> {
        if terms.is_empty() {
            return Vec::new();
        }
        let sets: Vec<std::collections::HashSet<usize>> =
            terms.iter().map(|t| self.search(t).into_iter().collect()).collect();
        let mut result = sets[0].clone();
        for s in &sets[1..] {
            result = result.intersection(s).copied().collect();
        }
        let mut sorted: Vec<usize> = result.into_iter().collect();
        sorted.sort_unstable();
        sorted
    }

    /// Python `InvertedIndex.vocabulary_size`.
    pub fn vocabulary_size(&self) -> usize {
        self.postings.len()
    }

    /// Python `InvertedIndex.term_frequency`.
    pub fn term_frequency(&self, term: &str) -> usize {
        self.postings.get(&term.to_lowercase()).map(|p| p.len()).unwrap_or(0)
    }
}
