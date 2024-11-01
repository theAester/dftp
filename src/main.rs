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

use std::env;
use std::process::exit;

mod cmd;
mod network;
mod files;
mod protocol;
mod compress;

use crate::cmd::{parse_args};
use crate::protocol::*;

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

