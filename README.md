# dftp
Direct File Transfer Protocol

Except that there is no protool and The file just gets shoved through a streaming socket.

Wrote this out of pure spite because i was infuriated by the fact that it didn't exist already.

Don't act like you didn't want it. You don't always want ssl and netcat can be very slow when transfering files(for me it was).

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
dftp -r [-p {listen_port_numer}=8086] [-f {output_file_name}]
```

then on the sending end type:
``` bash 
dftp {ip_address_of_recieving_end}[:recieve_port=8086] [-p {bind_port_number}=any] [-f {input_file_name}]
```
and watch this baby go.

## Notes
Explanatory error messages and usage hints are yet to come.

Default behavior is reading/writing from/to stdin/stdout for the sending/receiving end. Unless -f is specified.
