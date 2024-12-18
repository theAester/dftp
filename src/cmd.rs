extern crate getopts;

use getopts::{Options, HasArg, Occur};
use std::process::exit;

pub const DIR_SEND: i16 = 1;
pub const DIR_RECV: i16 = 0;
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn parse_args(argv:Vec<String>) -> Result<(i32, i16, String, String, bool), String>{
    let mut opts = Options::new();
    let mut port:i32 = -1;
    let mut direction:i16 = DIR_SEND;
    let mut filename:String = String::from("stdin");
    let mut addrstring = String::from("");
    let mut compress: bool = false;

    opts.opt("r", "recv", "act as recieving end", "recv", HasArg::No, Occur::Optional);
    opts.opt("p", "port", "use this port for self(default: 8086)", "port", HasArg::Yes, Occur::Optional);
    opts.opt("f", "file", "use this file instead of stdin/out", "file", HasArg::Yes, Occur::Optional);
    opts.opt("h", "help", "display this help message", "help", HasArg::No, Occur::Optional);
    opts.opt("v", "version", "displays dftp's build version", "help", HasArg::No, Occur::Optional);
    opts.opt("x",  "compress", "compressed transportation. must be specified on sender side.", "compress", HasArg::No, Occur::Optional);

    let appname = &argv[0];

    let matches = match opts.parse(&argv[1..]){
        Ok(m) => m,
        Err(m) => { return Err(format!("Error while parsing input arguments:\n    {}", m)); }
    };

    if matches.opt_present("h") {
        print_help(appname, opts);
        exit(0);
    }
    if matches.opt_present("v") {
        println!("{} v{}", appname, APP_VERSION);
        exit(0);
    }
    if matches.opt_present("r") {
        direction = DIR_RECV;
        //filename = String::from("default");
        port = 8086;
    }
    if matches.opt_present("p") {
        port = match matches.opt_str("p").expect("Unexpected error").parse::<i32>() {
            Ok(p) => p,
            Err(_) => { return Err("Error while parsing -p: argument is not a number".to_string()); }
        };
        if port < 1 || port > 65536 {
            return Err("Error while parsing -p: port number out of range".to_string());
        }
    }
    if matches.opt_present("f"){
        filename = matches.opt_str("f").expect("Unexpected Error");
    }
    if matches.opt_present("x"){
        compress = true;
    }
    if direction == DIR_SEND {
        if matches.free.len() > 2 || matches.free.len() < 1{
            return Err("Usage error: unexpected number of arguments. See --help for more info".to_string());
        }
        if matches.free.len() == 1 {
            // check if the port is specifically specified
            match matches.free[0].find(":") {
                Some(_) => {
                    addrstring = matches.free[0].clone();
                },
                None => {
                    addrstring = format!("{}:8086", matches.free[0]);
                }
            }
        }
        else if matches.free.len() == 2 {
            addrstring = format!("{}:{}", matches.free[0], matches.free[1]);
        }
        if !is_addr_string_valid(&addrstring) {
            return Err("Usage error: Invalid address specified. See --help for more info".to_string());
        }
    }
    if compress == true && direction == DIR_RECV {
        eprintln!("WARNING: you have specified -x on the receiving end. Take note that even though you have specified it, if the sender hasnt specified the flag your option will be ignored");
    }
    Ok((port, direction, filename, addrstring, compress))
}

fn print_help(appname: &str, opts: Options){
    let brief = format!("Usage: {} [OPTIONS] [ADDR]", appname);
    let usage = opts.usage(&brief);
    println!("{} v{}\n{}\nADDR =\t<IPv4 addr>:<port>\n", 
        appname, 
        APP_VERSION,
        usage);
}

fn is_addr_string_valid(addrstring: &String) -> bool {
    const STATE_IP1: u8 = 0;
    const STATE_IP2: u8 = 1;
    const STATE_IP3: u8 = 2;
    const STATE_IP4: u8 = 3;
    const STATE_PORT: u8 = 4;
    let mut state = STATE_IP1;
    let mut acc: String = String::new();

    for c in addrstring.chars() {
        match state {
            STATE_IP1 => {
                if c == '.' {
                    let parsed = acc.parse::<u8>();
                    match parsed {
                        Ok(_) => {
                            state = STATE_IP2;
                            acc = String::new()
                        },
                        Err(_) => {return false}
                    }
                } else {
                    acc.push(c)
                }
            },
            STATE_IP2 => {
                if c == '.' {
                    let parsed = acc.parse::<u8>();
                    match parsed {
                        Ok(_) => {
                            state = STATE_IP3;
                            acc = String::new();
                        },
                        Err(_) => {return false}
                    }
                } else {
                    acc.push(c)
                }
            },
            STATE_IP3 => {
                if c == '.' {
                    let parsed = acc.parse::<u8>();
                    match parsed {
                        Ok(_) => {
                            state = STATE_IP4;
                            acc = String::new();
                        },
                        Err(_) => {return false}
                    }
                } else {
                    acc.push(c)
                }
            },
            STATE_IP4 => {
                if c == ':' {
                    let parsed = acc.parse::<u8>();
                    match parsed {
                        Ok(_) => {
                            state = STATE_PORT;
                            acc = String::new();
                        },
                        Err(_) => {return false}
                    }
                } else {
                    acc.push(c)
                }
            },
            STATE_PORT => {
                acc.push(c)
            },
            5_u8..=u8::MAX => unreachable!(),
        }
    }

    let parsed = acc.parse::<u16>();
    match parsed {
        Ok(_) => true,
        Err(_) => false 
    }
}

