# dftp
Direct File Transfer Protocol

This is for when you need to quickly transfer a file between two devices over the network.

It is not secure (yet) but easier to use than ssh and sftp since it's a one-shot program and doesnt require configuring a server.
Moreover it has a very simple and understandable protocol.

It's just like how you may use netcat to transfer files however it's specialized for this purpose and is faster due to compression.

## Installation
This is a Cargo project. simply:
``` bash
cargo build --release
```
The bianry files will reside in ./target/release

You could also use 
``` bash
cargo install --path [install path]
```
to install it locally

## Usage
First, On the receiving end type:
``` bash
dftp -r [-p {listen_port_numer}=8086] [-f {output_file_name}=stdout]
```

then on the sending end type:
``` bash 
dftp {ip_address_of_recieving_end}[:recieve_port=8086] [-p {bind_port_number}=any] [-f {input_file_name}=stdin] [-x]
```

the `-x` flag tells the program to use xz compression over the network.

## Notes
~~Explanatory error messages and usage hints are yet to come.~~ DONE.
Support for transfering multiple files is the next task.

Default behavior is reading/writing from/to stdin/stdout for the sending/receiving end. Unless -f is specified.

Compression is disabled by default. use `-x` on the sending side to enable it.

