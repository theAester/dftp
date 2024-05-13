use std::io::{Read, Write};
use std::net::{TcpStream};
use std::io::ErrorKind::{WouldBlock, TimedOut};

pub const COMPAT_NUMBER: u8         = 1;

pub const SIMPLE_MSG_SENDER_ID: u8  = 0;
pub const SIMPLE_MSG_RECVER_ID: u8  = 1;
pub const SIMPLE_MSG_HS_ACK: u8     = 0b11111001;
pub const SIMPLE_MSG_PN_ACC: u8     = 0b00001001;
pub const SIMPLE_MSG_PN_DEN: u8     = 0b00001000;

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
            SIMPLE_MSG_PN_DEN => {},
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
            SIMPLE_MSG_PN_DEN => {},
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




