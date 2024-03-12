use std::fs;


pub fn scan_in_text(filename: &str) -> String {
    
    let s = fs::read_to_string(filename)
        .expect(format!("File: {filename} not found").as_str());
    return s;
}