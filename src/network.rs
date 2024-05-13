extern crate net2;

use std::net::TcpStream;
use net2::TcpBuilder;
use std::io::{Read, Write};

use crate::protocol::{handshake_send, handshake_recv};

pub fn build_send_stream(port:i32, addrstr:String) -> Result<TcpStream, String>{
    let builder = match TcpBuilder::new_v4(){
        Ok(s) => s,
        Err(m)=> {return Err(m.to_string()); }
    };
    if port != -1 {
        match builder.bind(("127.0.0.1", port as u16)) {
            Ok(_) => {},
            Err(m) => { return Err(format!("Cannot bind to port {}:\n    {}", port, m.to_string())); }
        }
    }
    /*
    match builder.connect(addrstr) {
        Ok(s) => s,
        Err(_) => { return Err(3); }
    }; */
    let mut stream = match builder.connect(addrstr) { //builder.to_tcp_stream() {
        Ok(s) => s,
        Err(m) => { return Err(format!("connection failed: {}", m.to_string())); }
    };
    return Ok(stream);
}

pub fn build_recv_stream(port:i32) -> Result<TcpStream, String>{
    let builder = match TcpBuilder::new_v4(){
        Ok(s) => s,
        Err(m)=> {return Err(m.to_string()); }
    };
    let builder = match builder.bind(("0.0.0.0", port as u16)){
        Ok(s) => s,
        Err(m) => {return Err(format!("Cannot bind to port {}: {}", port, m.to_string())); }
    };
    let listener = match builder.listen(10) {
        Ok(s) => s,
        Err(m) => { return Err(format!("Listening on port {} failed: {}", port, m.to_string())); }
    };
    let (mut recvr, _) = match listener.accept() {
        Ok(s) => s,
        Err(m) => { return Err(format!("Cannot accept peer: {}", m.to_string())); }
    };
    Ok(recvr)
}

