/************************************************************
 **************The Direct File Transter Protocol************
 ***********************************************************
 ****because sometimes security doesnt matter that much*****
 ******and net cat is just too slow for file transfer*******
 ***********************************************************
 ***********************************************************
 ***********************************************************
 ************************theAester**************************
 **********************************************************/

use std::io::{Write, Read, ErrorKind::ConnectionReset};
use std::env;
use std::process::exit;

mod cmd;
mod network;
mod files;
mod protocol;

use crate::cmd::{parse_args};
use crate::network::{build_send_stream, build_recv_stream};
use crate::files::{build_file_reader, build_file_writer};
//use crate::protocol::*;

fn send(port:i32, filename:String, addrstr:String, compress: bool){
    let mut sender = match build_send_stream(port, addrstr, compress) {
        Ok(s) => s,
        Err(m) => { eprintln!("Error while starting stream:\n  {}", m); exit(1); }
    };
    let mut reader = match build_file_reader(&filename){
        Ok(r) => r,
        Err(m) => { eprintln!("Error while reading file:\n  {}", m);exit(1); }
    };
    let mut bufflen:usize;
    let mut buff:[u8; 512] = [0; 512];
    loop{
        bufflen = reader.read(&mut buff).expect("Unexpected Error while reading from file. Aborting.");
        if bufflen == 0 { break; }
        sender.write_all(&buff[0..bufflen]).expect("Unexpected network error. Aborting.");
    }
}

fn recv(port:i32, filename:String, _compress: bool) {
    let mut recvr = match build_recv_stream(port) {
        Ok(s) => s,
        Err(m) => { eprintln!("Error while starting stream:\n  {}", m); exit(1); }
    };
    let mut writer = match build_file_writer(&filename){
        Ok(r) => r,
        Err(m) => { eprintln!("Error while writing to file:\n  {}", m); exit(1); }
    };
    let mut bufflen:usize;
    let mut buff:[u8; 512] = [0; 512];
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


fn main(){
    let argv:Vec<String> = env::args().collect();

    let (port, direction, filename, addrstring, compress) = match parse_args(argv){
        Ok(s) => s,
        Err(m) => { eprintln!("Error while parsing input arguments:\n  {}", m); exit(1); }
    };

    if direction == 1 {
        send(port, filename, addrstring, compress); 
    }else {
        recv(port, filename, compress);
    }
}

