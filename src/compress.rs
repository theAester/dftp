//extern crate xz2;

use std::io::{Read, Write};

//use xz2::write::XzEncoder;
//use xz2::read::XzDecoder;

use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use flate2::Compression;

pub fn wrap_compressor(writer: Box<dyn Write>) -> Box<dyn Write> {
    //Box::new(XzEncoder::new(writer, 3))
    Box::new(DeflateEncoder::new(writer, Compression::default()))
}

pub fn wrap_decompressor(reader: Box<dyn Read>) -> Box<dyn Read> {
    //Box::new(XzDecoder::new(reader))
    Box::new(DeflateDecoder::new(reader))
}
