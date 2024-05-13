use std::io::{Write, Read, ErrorKind::ConnectionReset};
use std::net::{TcpStream};
use std::io::ErrorKind::{WouldBlock, TimedOut};
use std::process::exit;

use crate::network::{build_send_stream, build_recv_stream};
use crate::files::{build_file_reader, build_file_writer};
use crate::compress::{wrap_compressor, wrap_decompressor};

pub const TRANSFER_BUFF_SIZE: usize = 262144; // 256 * 1024 bytes

pub const COMPAT_NUMBER: u8         = 1;

pub const SIMPLE_MSG_SENDER_ID: u8  = 0;
pub const SIMPLE_MSG_RECVER_ID: u8  = 1;
pub const SIMPLE_MSG_HS_ACK: u8     = 0b11111001;
pub const SIMPLE_MSG_PN_ACC: u8     = 0b00001001;
pub const SIMPLE_MSG_PN_DEC: u8     = 0b00001000;

pub const PT_FLAG_COMPRESS: u8      = 1;

pub const FH_TYPE_FILE: u8          = 0;
pub const FH_TYPE_DIR: u8           = 1;

struct Simple{
    content: u8,
}

struct ProtocolTable{
    compat_num: u8,
    //flags
    compressed: bool,
}

struct FileHeader{
    length: u32,
    file_type: u8,
    name: String,
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
        if flags & PT_FLAG_COMPRESS != 0 { self.compressed = true; } else { self.compressed = false }
        // future implementation
        Ok(2) // 2 bytes consumed
    }
}

impl TcpShovable for FileHeader{
    fn shove(&self, stream: &mut TcpStream) -> Result<usize, String> {
        let name = &self.name;
        let buflen = 9 + name.len();
        let mut buf: Vec<u8> = vec![0; buflen];
        let len = self.length;
        // big endian
        buf[0] = ((len >> 3)       ) as u8;
        buf[1] = ((len >> 2) & 0xff) as u8;
        buf[2] = ((len >> 1) & 0xff) as u8;
        buf[3] = ((len     ) & 0xff) as u8;

        buf[4] = self.file_type;
        // get the string, write the len first 
        // and then the string itself.
        let name = name.as_bytes();
        let len = name.len();
        buf[5] = ((len >> 3)       ) as u8;
        buf[6] = ((len >> 2) & 0xff) as u8;
        buf[7] = ((len >> 1) & 0xff) as u8;
        buf[8] = ((len     ) & 0xff) as u8;
        let mut i: usize = 9;
        for b in name {
            buf[i] = *b;
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
        len <<= 1; len += buf[0] as u32;
        len <<= 1; len += buf[1] as u32;
        len <<= 1; len += buf[2] as u32;
        len <<= 1; len += buf[3] as u32;
        self.length = len;
        match buf[4] {
            FH_TYPE_FILE | 
            FH_TYPE_DIR => {},
            _ => { return Err(format!("Error when unpacking FH, invalid file type {}", buf[4])); }
        }
        self.file_type = buf[4];
        let mut len: u32 = 0;
        len <<= 1; len += buf[5] as u32;
        len <<= 1; len += buf[6] as u32;
        len <<= 1; len += buf[7] as u32;
        len <<= 1; len += buf[8] as u32;
        let len = len as usize;
        let buf2: Vec<u8> = vec![0; len];
        match stream.read_exact(&mut buf[..]) {
        Err(e) => {
            match e.kind() {
                WouldBlock | TimedOut => {return Err("requst timeout".to_string());}
                _ => { panic!("something is seriously wrong"); },
            }
        },
        Ok(_) => {},
        }
        let name = String::from_utf8(buf2).expect("expecting valid utf8");
        self.name = name;
        Ok(9 + len) // 9 + len bytes consumed
    }
}

impl Simple {
    pub fn new() -> Simple{
        Simple{content: 0u8}
    }
}

impl ProtocolTable {
    pub fn new() -> ProtocolTable{
        ProtocolTable{
            compat_num: COMPAT_NUMBER,
            compressed: false,
        }
    }
}

impl FileHeader{
    pub fn new() -> FileHeader{
        FileHeader{
            length: 0,
            file_type: FH_TYPE_FILE,
            name: String::new(),
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
    let mut message = Simple::new();
    message.content = SIMPLE_MSG_SENDER_ID;
    message.shove(peer)?;

    // wait to receive a recver id handshake
    let mut message = Simple::new();
    message.pull(peer)?;
    if message.content == SIMPLE_MSG_SENDER_ID {
        return Err("Cannot perform handshake. The peer is also a sender".to_string());
    }
    else if message.content != SIMPLE_MSG_RECVER_ID {
        return Err("Malfunction 1".to_string());
    }
    
    // send ack
    let mut message = Simple::new();
    message.content = SIMPLE_MSG_HS_ACK;
    message.shove(peer)?;

    Ok(())
}

pub fn handshake_recv(peer: &mut TcpStream) -> Result<(), String> {
    // wait to recv a sender id handshake
    let mut message = Simple::new();
    message.pull(peer)?;
    if message.content != SIMPLE_MSG_SENDER_ID {
        return Err("Malfunction 2".to_string());
    }

    // send a recver id message
    let mut message = Simple::new();
    message.content = SIMPLE_MSG_RECVER_ID;
    message.shove(peer)?;

    // wait to recv ack
    let mut message = Simple::new();
    message.pull(peer)?;
    if message.content != SIMPLE_MSG_HS_ACK {
        return Err("Malfunction 3".to_string());
    }

    Ok(())
}

fn protocol_adjust_send(mut peer: TcpStream, compress: bool) -> Result<Box<dyn Write>, String>{
    // for now compression is the only protocol adjustment

    // craft a protocol table message and send it
    let mut message = ProtocolTable::new();
    message.compressed = compress;
    message.shove(&mut peer)?;

    // wait for a negotiation response
    let mut message = Simple::new();
    message.pull(&mut peer)?;
    if message.content == SIMPLE_MSG_PN_DEC {
        return Err("Protocol negotiation failed: peer declined.\nOne of you needs to update their dftp.".to_string());
    }
    if message.content != SIMPLE_MSG_PN_ACC {
        return Err("Malfunction 4".to_string());
    }

    // here the peer has accepted out protocol negotiation
    // time to upgrade protocol    
    
    let writer: Box<dyn Write> = Box::new(peer);
    let writer = if compress {
        wrap_compressor(writer)
    } else { writer };

    // more protocol upgrades here
    
    Ok(Box::new(writer))
}

fn protocol_adjust_recv(mut peer: TcpStream) -> Result<Box<dyn Read>, String>{
    // wait for a protocol table
    let mut message = ProtocolTable::new();
    match message.pull(&mut peer) {
        Ok(_) => {
            let mut decl = Simple::new();
            decl.content = SIMPLE_MSG_PN_ACC;
            decl.shove(&mut peer)?;
        },
        Err(m) => {
            // protocol request is fucked. send decline message
            let mut decl = Simple::new();
            decl.content = SIMPLE_MSG_PN_DEC;
            decl.shove(&mut peer)?;
            return Err(m);
        }
    }

    // if we're here it means that protocl negotiation was successful.
    // time to upgrade protocol

    let reader: Box<dyn Read> = Box::new(peer);
    let reader = if message.compressed {
        wrap_decompressor(reader)
    } else { reader };

    // more protocol upgrades here
    
    Ok(Box::new(reader))
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
    let mut sender = match protocol_adjust_send(sender, compress) {
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
    let mut recvr = match protocol_adjust_recv(recvr) {
        Ok(s) => s,
        Err(m) => { eprintln!("{}", m); exit(1); }
    };
    let mut writer = match build_file_writer(&filename){
        Ok(r) => r,
        Err(m) => { eprintln!("Error while writing to file:\n  {}", m); exit(1); }
    };
    let mut bufflen:usize;
    let mut buff:[u8; TRANSFER_BUFF_SIZE] = [0; TRANSFER_BUFF_SIZE];
    loop{
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
        writer.write_all(&buff[0..bufflen]).expect("Unexpected network error. Aborting.");
        if filename == "stdout" { writer.flush().expect("wtf?"); }
    }
}




