use std::io::{Write, Read, ErrorKind::ConnectionReset, Error};
use std::iter::Flatten;
use std::net::{TcpStream};
use std::io::ErrorKind::{WouldBlock, TimedOut, BrokenPipe};
use std::fs::File;
use std::path::Path;
use std::process::exit;
use std::time::SystemTime;
use std::fmt::Write as fWrite;

use sha2::{Digest, Sha256};

use crate::network::{build_send_stream, build_recv_stream};
use crate::files::{
    build_file_reader, 
    build_file_writer,
    defer_kind,
};
use crate::compress::{wrap_compressor, wrap_decompressor};

pub const TRANSFER_BUFF_SIZE: usize = 262144; // 256 * 1024 bytes

pub const COMPAT_NUMBER: u8         = 2;

pub const SIMPLE_MSG_SENDER_ID: u8  = 0;
pub const SIMPLE_MSG_RECVER_ID: u8  = 1;
pub const SIMPLE_MSG_HS_ACK: u8     = 0b11111001;
pub const SIMPLE_MSG_PN_ACC: u8     = 0b00001001;
pub const SIMPLE_MSG_PN_DEC: u8     = 0b00001000;

pub const PT_FLAG_COMPRESS: u8      = 1;
pub const PT_FLAG_FILE: u8          = 2;

pub const FH_TYPE_FILE: u8          = 0;
pub const FH_TYPE_DIR: u8           = 1;

struct Simple{
    content: u8,
}

struct ProtocolTable{
    compat_num: u8,
    //flags
    compressed: bool,
    isfile: bool,
}

struct FileHeader{
    length: u32, // file size not name length!
    file_type: u8,
    name: String,
    hash: [u8; 32],
}

trait TcpShovable {
    fn shove(&self, stream: &mut TcpStream) -> Result<usize, String>;
    fn pull(&mut self, stream: &mut TcpStream) -> Result<usize, String>;
}

impl TcpShovable for Simple {
    fn shove(&self, stream: &mut TcpStream) -> Result<usize, String> {
        let mut buf:[u8; 1] = [0; 1];
        match self.content{
            SIMPLE_MSG_SENDER_ID |
            SIMPLE_MSG_RECVER_ID | 
            SIMPLE_MSG_HS_ACK    |
            SIMPLE_MSG_PN_ACC    |
            SIMPLE_MSG_PN_DEC => {},
            _ => { return Err("Invalid value for simple message encountered while packing".to_string()); }
        }
        buf[0] = self.content;
        match stream.write_all(&buf) {
            Err(e) => {
                match e.kind() {
                    WouldBlock | TimedOut => {return Err("requst timeout".to_string());}
                    _ => { panic!("something is seriously wrong"); },
                }
            },
            Ok(_) => {},
        }
        Ok(1) // 1 byte written
    }
    fn pull(&mut self, stream: &mut TcpStream) -> Result<usize, String> {
        let mut buf:[u8; 1] = [0; 1];
        match stream.read_exact(&mut buf) {
            Err(e) => {
                match e.kind() {
                    WouldBlock | TimedOut => {return Err("requst timeout".to_string());}
                    _ => { panic!("something is seriously wrong"); },
                }
            },
            Ok(_) => {},
        }
        match buf[0]{
            SIMPLE_MSG_SENDER_ID |
            SIMPLE_MSG_RECVER_ID | 
            SIMPLE_MSG_HS_ACK    |
            SIMPLE_MSG_PN_ACC    |
            SIMPLE_MSG_PN_DEC => {},
            _ => { return Err("Invalid value for simple message encountered while unpacking".to_string()); }
        }
        self.content = buf[0];
        Ok(1) // 1 bytes consumed
    }
}

impl TcpShovable for ProtocolTable{
    fn shove(&self, stream: &mut TcpStream) -> Result<usize, String> {
        let mut buf:[u8; 2] = [0; 2];
        buf[0] = self.compat_num;
        let mut flags: u8 = 0;
        if self.compressed {
            flags |= PT_FLAG_COMPRESS;
        }
        if self.isfile {
            flags |= PT_FLAG_FILE;
        }
        // future implementation
        buf[1] = flags;
        match stream.write_all(&buf) {
            Err(e) => {
                match e.kind() {
                    WouldBlock | TimedOut => {return Err("requst timeout".to_string());}
                    _ => { panic!("something is seriously wrong"); },
                }
            },
            Ok(_) => {},
        }
        Ok(2) // 2 bytes written
    }
    fn pull(&mut self, stream: &mut TcpStream) -> Result<usize, String> {
        let mut buf:[u8; 2] = [0; 2];
        match stream.read_exact(&mut buf) {
            Err(e) => {
                match e.kind() {
                    WouldBlock | TimedOut => {return Err("requst timeout".to_string());}
                    _ => { panic!("something is seriously wrong"); },
                }
            },
            Ok(_) => {},
        }
        let compat_num = buf[0];
        if compat_num != COMPAT_NUMBER {
            return Err (format!("Incompatible protocol versions. Ours is {}. Theirs is {}", 
                                COMPAT_NUMBER, compat_num));
        }
        self.compat_num = compat_num;
        let flags = buf[1];
        self.compressed = (flags & PT_FLAG_COMPRESS) != 0;
        self.isfile = (flags & PT_FLAG_FILE) != 0;
        // future implementation
        Ok(2) // 2 bytes consumed
    }
}

impl TcpShovable for FileHeader{
    fn shove(&self, stream: &mut TcpStream) -> Result<usize, String> {
        let name = &self.name;
        let buflen = 4 + 1 + 4 + 32 + name.len();
        let mut buf: Vec<u8> = vec![0; buflen];
        let len = self.length;
        // big endian
        buf[0] = ((len >> 24)       ) as u8;
        buf[1] = ((len >> 16) & 0xff) as u8;
        buf[3] = ((len >> 8) & 0xff) as u8;
        buf[3] = ((len     ) & 0xff) as u8;

        buf[4] = self.file_type;
        // get the string, write the len first 
        // and then the string itself.
        let name = name.as_bytes();
        let len = name.len();
        buf[5] = ((len >> 24)       ) as u8;
        buf[6] = ((len >> 16) & 0xff) as u8;
        buf[7] = ((len >> 8) & 0xff) as u8;
        buf[8] = ((len     ) & 0xff) as u8;
        let mut i: usize = 9;
        for b in name {
            buf[i] = *b;
            i += 1;
        }
        for j in 0..32 {
            buf[i] = self.hash[j];
            i += 1;
        }
        let buf = &buf[..];
        assert!(i == buflen);
        match stream.write_all(buf){
            Err(e) => {
                match e.kind() {
                    WouldBlock | TimedOut => {return Err("requst timeout".to_string());}
                    _ => { panic!("something is seriously wrong"); },
                }
            },
            Ok(_) => {},
        }
        Ok(i) // i bytes written
    }
    
    fn pull(&mut self, stream: &mut TcpStream) -> Result<usize, String> {
        let mut buf:[u8; 9] = [0; 9];
        match stream.read_exact(&mut buf){
            Err(e) => {
                match e.kind() {
                    WouldBlock | TimedOut => {return Err("requst timeout".to_string());}
                    _ => { panic!("something is seriously wrong"); },
                }
            },
            Ok(_) => {},
        }
        let mut len: u32 = 0;
        len <<= 8; len += buf[0] as u32;
        len <<= 8; len += buf[1] as u32;
        len <<= 8; len += buf[2] as u32;
        len <<= 8; len += buf[3] as u32;
        self.length = len;
        match buf[4] {
            FH_TYPE_FILE | 
            FH_TYPE_DIR => {},
            _ => { return Err(format!("Error when unpacking FH, invalid file type {}", buf[4])); }
        }
        self.file_type = buf[4];
        let mut len: u32 = 0;
        len <<= 8; len += buf[5] as u32;
        len <<= 8; len += buf[6] as u32;
        len <<= 8; len += buf[7] as u32;
        len <<= 8; len += buf[8] as u32;
        let len = len as usize;
        println!("size: {}", len);
        let mut buf2: Vec<u8> = vec![0u8; len];
        match stream.read_exact(&mut buf2[..]) {
        Err(e) => {
            match e.kind() {
                WouldBlock | TimedOut => {return Err("requst timeout".to_string());}
                _ => { panic!("something is seriously wrong"); },
            }
        },
        Ok(_) => {},
        };
        let name = String::from_utf8(buf2).expect("expecting valid utf8");
        self.name = name;
        let mut hash = [0u8; 32];
        match stream.read_exact(&mut hash){
        Err(e) => {
            match e.kind() {
                WouldBlock | TimedOut => {return Err("requst timeout".to_string());}
                _ => { panic!("something is seriously wrong"); },
            }
        },
        Ok(_) => {},
        };
        self.hash = hash;
        Ok(9 + len + 32) // 9 + len bytes consumed
    }
}

impl Simple {
    pub fn default() -> Simple{
        Simple{content: 0u8}
    }
}

impl ProtocolTable {
    pub fn default() -> ProtocolTable{
        ProtocolTable{
            compat_num: COMPAT_NUMBER,
            compressed: false,
            isfile: false
        }
    }
}

impl FileHeader{
    pub fn default() -> FileHeader{
        FileHeader{
            length: 0,
            file_type: FH_TYPE_FILE,
            name: String::new(),
            hash: [0u8; 32],
        }
    }
}

enum Message {
    Simple,
    ProtocolTable,
    FileHeader
}

pub fn handshake_send(peer: &mut TcpStream) -> Result<(), String> {
    // send a sender id handshake message
    let mut message = Simple::default();
    message.content = SIMPLE_MSG_SENDER_ID;
    message.shove(peer)?;

    // wait to receive a recver id handshake
    let mut message = Simple::default();
    message.pull(peer)?;
    if message.content == SIMPLE_MSG_SENDER_ID {
        return Err("Cannot perform handshake. The peer is also a sender".to_string());
    }
    else if message.content != SIMPLE_MSG_RECVER_ID {
        return Err("Malfunction 1".to_string());
    }
    
    // send ack
    let mut message = Simple::default();
    message.content = SIMPLE_MSG_HS_ACK;
    message.shove(peer)?;

    Ok(())
}

pub fn handshake_recv(peer: &mut TcpStream) -> Result<(), String> {
    // wait to recv a sender id handshake
    let mut message = Simple::default();
    message.pull(peer)?;
    if message.content != SIMPLE_MSG_SENDER_ID {
        return Err("Malfunction 2".to_string());
    }

    // send a recver id message
    let mut message = Simple::default();
    message.content = SIMPLE_MSG_RECVER_ID;
    message.shove(peer)?;

    // wait to recv ack
    let mut message = Simple::default();
    message.pull(peer)?;
    if message.content != SIMPLE_MSG_HS_ACK {
        return Err("Malfunction 3".to_string());
    }

    Ok(())
}

fn protocol_adjust_send(mut peer: TcpStream, filename: &String, compress: bool) -> Result<Box<dyn Write>, String>{
    let isfile = defer_kind(filename);

    // craft a protocol table message and send it
    let mut message = ProtocolTable::default();
    message.compressed = compress;
    message.isfile = isfile;
    message.shove(&mut peer)?;

    // wait for a negotiation response
    let mut message = Simple::default();
    message.pull(&mut peer)?;
    if message.content == SIMPLE_MSG_PN_DEC {
        return Err("Protocol negotiation failed: peer declined.\nOne of you needs to update their dftp.".to_string());
    }
    if message.content != SIMPLE_MSG_PN_ACC {
        return Err("Malfunction 4".to_string());
    }

    // here the peer has accepted out protocol negotiation

    // send file header if necesary
    if isfile {
        match send_file_header(&mut peer, filename) {
            Ok(_) => {},
            Err(m) => {return Err(format!("Error while opening file: {:}", m.to_string()));}
        };
    }


    // time to upgrade protocol    
    
    let writer: Box<dyn Write> = Box::new(peer);
    let writer = if compress {
        wrap_compressor(writer)
    } else { writer };

    // more protocol upgrades here
    
    Ok(Box::new(writer))
}

fn protocol_adjust_recv(mut peer: TcpStream) -> Result<(Box<dyn Read>,
                                                        Option<FileHeader>,
                                                        ProtocolTable), String>{
    // wait for a protocol table
    let mut message = ProtocolTable::default();
    match message.pull(&mut peer) {
        Ok(_) => {
            let mut decl = Simple::default();
            decl.content = SIMPLE_MSG_PN_ACC;
            decl.shove(&mut peer)?;
        },
        Err(m) => {
            // protocol request is fucked. send decline message
            let mut decl = Simple::default();
            decl.content = SIMPLE_MSG_PN_DEC;
            decl.shove(&mut peer)?;
            return Err(m);
        }
    }

    // if we're here it means that protocl negotiation was successful.

    // recv file header if necessary
    let mut fileheader: Option<FileHeader> = None;
    if message.isfile {
        fileheader = match recv_file_header(&mut peer) {
            Ok(e) => Some(e),
            Err(m) => {return Err(format!("Error while receiving file header: {:}", m.to_string()));}
        };
    }

    // time to upgrade protocol

    let reader: Box<dyn Read> = Box::new(peer);
    let reader = if message.compressed {
        wrap_decompressor(reader)
    } else { reader };

    // more protocol upgrades here
    
    Ok((Box::new(reader), fileheader, message))
}

fn send_file_header(peer: &mut TcpStream, filename: &String) -> Result<(), Error>{
    let mut header = FileHeader::default();
    header.file_type = FH_TYPE_FILE; // dir sending is not available for now...
    header.name = String::from(Path::new(filename).file_name().unwrap().to_str().unwrap());
    
    let mut file = File::open(filename)?;
    let size = file.metadata()?.len() as u32;
    header.length = size;
    let mut sha = Sha256::new();
    std::io::copy(&mut file, &mut sha)?;
    let hash = sha.finalize();
    assert!(Sha256::output_size() == 32usize);
    assert!(hash.len() == 32usize);
    for i in 0..32{
        header.hash[i] = hash[i];
    }

    header.shove(peer).unwrap();
    Ok(())
}

fn recv_file_header(peer: &mut TcpStream) -> Result<FileHeader, Error>{
    let mut header = FileHeader::default();
    match header.pull(peer) {
        Ok(_) => {},
        Err(e) => {
            return Err(Error::new(BrokenPipe, e));
        }
    };
    Ok(header)
}

fn stringify_hash(hash: &[u8]) -> String {
    let mut s = String::new();
    for i in 0..32 {
        write!(&mut s, "{:02x}", hash[i]).expect("should be able to write to string");
    }
    return s;
}

fn print_file_info(filename: &String, fileheader: &Option<FileHeader>, compressed: bool){
    if fileheader.is_none() {return};
    if filename == "stdout" {return}; // dont fuck up the output!!
    let fileheader = fileheader.as_ref().unwrap();
    println!("Receiving file: {} [{:}]", fileheader.name, stringify_hash(&fileheader.hash));
    println!("Writing to: {}", filename);
    if compressed {
        println!("Compressed stream");
    }
    println!();
}


pub fn send(port:i32, filename:String, addrstr:String, compress: bool){
    let mut sender = match build_send_stream(port, addrstr) {
        Ok(s) => s,
        Err(m) => { eprintln!("Error while starting stream:\n  {}", m); exit(1); }
    };
    match handshake_send(&mut sender) {
        Ok(_) => {},
        Err(s) => { eprintln!("Handshake failed: {}", s); exit(1); }
    };
    let mut sender = match protocol_adjust_send(sender, &filename, compress) {
        Ok(s) => s,
        Err(m) => { eprintln!("{}", m); exit(1); }
    };
    let mut reader = match build_file_reader(&filename){
        Ok(r) => r,
        Err(m) => { eprintln!("Error while reading file:\n  {}", m);exit(1); }
    };
    let mut bufflen:usize;
    let mut buff:[u8; TRANSFER_BUFF_SIZE] = [0; TRANSFER_BUFF_SIZE];
    loop{
        bufflen = reader.read(&mut buff).expect("Unexpected Error while reading from file. Aborting.");
        if bufflen == 0 { break; }
        sender.write_all(&buff[0..bufflen]).expect("Unexpected network error. Aborting.");
    }
}

pub fn recv(port:i32, filename:String, _compress: bool) {
    let mut recvr = match build_recv_stream(port) {
        Ok(s) => s,
        Err(m) => { eprintln!("Error while starting stream:\n  {}", m); exit(1); }
    };
    match handshake_recv(&mut recvr) {
        Ok(_) => {},
        Err(s) => { eprintln!("Handshake failed: {}", s); exit(1); }
    };
    let (mut recvr, fileheader, pt_header) = match protocol_adjust_recv(recvr) {
        Ok(s) => s,
        Err(m) => { eprintln!("{}", m); exit(1); }
    };
    let mut filename = filename;
    if fileheader.is_some() && filename == "default" {
        filename = fileheader.as_ref().unwrap().name.clone();
    }
    let mut writer = match build_file_writer(&filename){
        Ok(r) => r,
        Err(m) => { eprintln!("Error while writing to file:\n  {}", m); exit(1); }
    };
    let mut bufflen:usize = 0;
    let mut buff:[u8; TRANSFER_BUFF_SIZE] = [0; TRANSFER_BUFF_SIZE];
    print_file_info(&filename, &fileheader, pt_header.compressed);
    let mut total: u32 = 0;
    let mut counter = 0;
    let mut bufflen_acc = 0;
    let mut now = SystemTime::now();
    loop{
        total += bufflen as u32;
        bufflen = match recvr.read(&mut buff) {
            Ok(n) => n,
            Err(m) => {
                match m.kind() {
                    ConnectionReset => {
                        eprintln!("Connection closed by peer");
                        break;
                    },
                    _ => {
                        panic!("{}", m);
                    },
                }
            }
        };
        if bufflen == 0 { break; }
        if counter == 0 {
            if let Some(fh) = fileheader.as_ref() {
                let micros = now.elapsed().unwrap().as_micros() as u64;
                let speed = (bufflen_acc * 1_000_000) as f64 / micros as f64;
                let speed = speed / 1024f64;
                print_status(speed, fh.length as f32 / (1024 * 1024) as f32, total as f32 / (1024 * 1024) as f32);
                bufflen_acc = 0;
            now = SystemTime::now();

            }
        }
        writer.write_all(&buff[0..bufflen]).expect("Unexpected network error. Aborting.");
        if filename == "stdout" { writer.flush().expect("wtf?"); }
        bufflen_acc += bufflen;
        counter += 1;
        if counter == 80 { counter = 0; }
    }
}

fn print_status(speed: f64, length: f32, total: f32) {
    static mut SIZE: usize = 0;
    unsafe {
        if SIZE > 0 {
            print!("\r{}", " ".repeat(SIZE));
            print!("\r");
        }
    }

    let ratio = total as f32 / length as f32;
    let percent = ratio * 100f32;
    let spaced = ratio * 20f32;
    let mut tiled = spaced as u32;
    let remain = spaced - tiled as f32;
    if remain >= 0.5 {
        tiled += 1;
    }
    
    let mut eq = String::new();
    for _ in 0..tiled {
        eq += "=";
    }
    for _ in 0..(20 - tiled) {
        eq += " ";
    }
    
    assert!(eq.len() == 20);
    let s = format!(
        "{:.1}% [{:}]\t{:.2} KiB/s\t{:.2}/{:.2} MiB",
        percent, eq, speed, total, length
    );
    
    unsafe {
        SIZE = s.len();
    }

    print!("{}", s);
    std::io::stdout().flush().unwrap(); // Flush the output to ensure immediate printing
}




