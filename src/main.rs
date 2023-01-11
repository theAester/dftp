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

extern crate getopts;
extern crate net2;

use getopts::{Options, HasArg, Occur};
use std::net::TcpStream;
use net2::TcpBuilder;
use std::io;
use std::io::{Write, Read, BufReader, BufWriter, BufRead};
use std::fs::File;
use std::env;
use std::str;
use std::result::Result;

fn parse_args(argv:Vec<String>) -> Result<(i32, i16, String, String), i16>{
    let mut opts = Options::new();
    let mut port:i32 = -1;
    let mut direction:i16 = 1; // for send
    let mut filename:String = String::from("stdin");
    let mut addrstring = String::from("");

    opts.opt("r", "recv", "act as recieving end", "recv", HasArg::No, Occur::Optional);
    opts.opt("p", "port", "use this port for self(default for receiving: 8086, default for sending: any)", "port", HasArg::Yes, Occur::Optional);
    opts.opt("f", "file", "use this file instead of stdin/out", "file", HasArg::Yes, Occur::Optional);

    let matches = match opts.parse(&argv[1..]){
        Ok(m) => m,
        Err(_) => { return Err(1); }
    };
    if matches.opt_present("r") {
        direction = 0;
        filename = String::from("stdout");
        port = 8086;
    };
    if matches.opt_present("p") {
        port = match matches.opt_str("p").expect("Unexpected error").parse::<i32>() {
            Ok(p) => p,
            Err(_) => { return Err(1); }
        }
    };
    if matches.opt_present("f"){
        filename = matches.opt_str("f").expect("Unexpected Error");
    }
    if matches.free.len() == 0 && direction == 1{
        return Err(1);
    } 
    if direction == 1{
        if matches.free.len() == 2{
            addrstring = matches.free[0].clone() + &matches.free[1].clone();
        }else {match matches.free[0].find(":"){
            Some(_) => {addrstring = matches.free[0].clone(); },
            None => { addrstring = matches.free[0].clone() + &":8086"; } 
        };}
    }
    Ok((port, direction, filename, addrstring))
}

fn build_send_stream(port:i32, addrstr:String) -> Result<TcpStream, i16>{
    let builder = match TcpBuilder::new_v4(){
        Ok(s) => s,
        Err(_)=> {return Err(2); }
    };
    if port != -1 {
        builder.bind(("127.0.0.1", port as u16)).expect("Unexpected Error with binding");
    }
    /*
    match builder.connect(addrstr) {
        Ok(s) => s,
        Err(_) => { return Err(3); }
    }; */
    return match builder.connect(addrstr) { //builder.to_tcp_stream() {
        Ok(s) => Ok(s),
        Err(m) => { eprint!("{}", m.to_string()); Err(4) }
    };
}

fn build_file_reader(filename:String) -> Result<Box<dyn BufRead>, i16>{
    if filename == "stdin" {
        return Ok(Box::new(BufReader::new(io::stdin())));
    }
    let file = match File::open(filename) {
        Ok(f) => f,
        Err(_) => {return Err(1); }
    };
    Ok(Box::new(BufReader::new(file)))
}

fn send(port:i32, filename:String, addrstr:String) -> Result<(), i16>{
    let mut sender = match build_send_stream(port, addrstr) {
        Ok(s) => s,
        Err(m) => { return Err(m); }
    };
    let mut reader = match build_file_reader(filename){
        Ok(r) => r,
        Err(_) => {return Err(2); }
    };
    let mut bufflen:usize;
    let mut buff:[u8; 512] = [0; 512];
    loop{
        bufflen = reader.read(&mut buff).expect("Unexpected Error while reading from file. Aborting.");
        if bufflen == 0 { break; }
        sender.write_all(&buff[0..bufflen]).expect("Unexpected network error. Aborting.");
    }
    Ok(())
}

fn build_file_writer(filename:String) -> Result<Box<dyn Write>, i16>{
    if filename == "stdout" {
        return Ok(Box::new(BufWriter::new(io::stdout())));
    }
    let file = match File::open(filename) {
        Ok(f) => f,
        Err(_) => {return Err(1); }
    };
    Ok(Box::new(BufWriter::new(file)))
}

fn build_recv_stream(port:i32) -> Result<TcpStream, i16>{
    let builder = match TcpBuilder::new_v4(){
        Ok(s) => s,
        Err(_)=> {return Err(2); }
    };
    let builder = builder.bind(("0.0.0.0", port as u16)).expect("Unexpected Error with binding");
    let listener = match builder.listen(10) {
        Ok(s) => s,
        Err(_) => { return Err(4); }
    };
    let (recvr, _) = match listener.accept() {
        Ok(s) => s,
        Err(_) => { return Err(3); }
    };
    Ok(recvr)
}

fn recv(port:i32, filename:String) -> Result<(), i16>{
    let mut recvr = match build_recv_stream(port) {
        Ok(s) => s,
        Err(m) => { return Err(m); }
    };
    let mut writer = match build_file_writer(filename){
        Ok(r) => r,
        Err(_) => {return Err(2); }
    };
    let mut bufflen:usize;
    let mut buff:[u8; 512] = [0; 512];
    loop{
        bufflen = recvr.read(&mut buff).expect("Unexpected Error while reading from file. Aborting.");
        if bufflen == 0 { break; }
        writer.write_all(&buff[0..bufflen]).expect("Unexpected network error. Aborting.");
    }
    Ok(())
}


fn main() -> Result<(), i16>{
    const ERRSTR:&str = "I dont understand a word of what youre saying!";

    let argv:Vec<String>= env::args().collect();

    let (port, direction, filename, addrstring) = match parse_args(argv){
        Ok(s) => s,
        Err(_) => {panic!("{}", ERRSTR);}
    };

    println!("{}", port);
    println!("{}", addrstring);

    if direction == 1 {
        return send(port, filename, addrstring); 
    }else {
        return recv(port, filename);
    }
    /*let mut sender = net::TcpStream::connect(argv[1].clone()).unwrap_or_else( |_| {
        panic!("{}", ERRSTR);
    });
    let file = File::open(argv[2].clone()).unwrap_or_else( |_| { 
        panic!("{}\njk I do understand the IP i just cant open the file.", ERRSTR);
    });
    let mut reader = BufReader::new(file);
    let mut buff:[u8;512] = [0;512];
    let mut read_bytes:usize;
    loop{ 
        read_bytes = reader.read(&mut buff).expect("Unexpected Error!");
        if read_bytes == 0 { break; }
        sender.write_all(&buff[0..read_bytes]).expect("Network error");
    }
    println!("done");*/
}

