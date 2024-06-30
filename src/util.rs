use std::path::{MAIN_SEPARATOR, MAIN_SEPARATOR_STR};

fn rfind_utf8(s: &str, char: char) -> Option<usize> {
    if let Some(rev_pos) = s.chars().rev().position(|c| c == char) {
        Some(s.chars().count() - rev_pos - 1)
    } else {
        None
    }
}

pub fn extract_lowest_path_item(path: &str) -> String {
    if path == "/" {
        path.to_string()
    } else {
        path.split(MAIN_SEPARATOR_STR)
            .filter(|path1| path1 != &"")
            .last()
            .unwrap()
            .to_string()
    }
}

pub fn remove_lowest_path_item(path: &str) -> String {
    path.chars()
        .take(rfind_utf8(path, MAIN_SEPARATOR).unwrap())
        .collect()
}
