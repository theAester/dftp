use std::io;
use std::io::{Write, BufReader, BufWriter, BufRead};
use std::fs::File;

pub fn defer_kind(filename: &String) -> bool {
    return !(filename == "stdin");
}

pub fn build_file_reader(filename:&String) -> Result<Box<dyn BufRead>, String>{
    if filename == "stdin" {
        return Ok(Box::new(BufReader::new(io::stdin())));
    }
    let file = match File::open(filename) {
        Ok(f) => f,
        Err(m) => { return Err(format!("Error opening file {} for reading: {}", filename, m.to_string())); }
    };
    Ok(Box::new(BufReader::new(file)))
}

pub fn build_file_writer(filename:&String) -> Result<Box<dyn Write>, String>{
    if filename == "stdin" {
        return Ok(Box::new(BufWriter::new(io::stdout())));
    }
    let file = match File::create(filename) {
        Ok(f) => f,
        Err(m) => { return Err(format!("Error opening file {} for writing: {}", filename, m.to_string())); }
    };
    Ok(Box::new(BufWriter::new(file)))
}

