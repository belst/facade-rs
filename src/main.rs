use std::str;
use std::fmt::Write;
use std::thread;
use std::net::UdpSocket;

fn toHexString(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &byte in bytes {
        write!(&mut s, "{:X} ", byte).unwrap();
    }
    s
}

fn main() {
    
    const PREFIX: &'static [u8; 4] = b"\xFF\xFF\xFF\xFF";
    
    println!("Spinning up server.");
    let socket = match UdpSocket::bind("127.0.0.1:5555") {
        Ok(s) => {
            println!("Listening on: {}", s.local_addr().unwrap());
            s
        },
        Err(e) => panic!("Could not bind socket: {}", e)
    };

    let mut buf = [0; 2048];

    loop {
        match socket.recv_from(&mut buf) {
            Ok((amt, src)) => {
                if buf[0 .. 4] != *PREFIX {
                    println!("Invalid Prefix");
                    continue;
                };
                thread::spawn(move || {
                    println!("new thread Spawned.");
                    println!("amt: {}", amt);
                    println!("src: {}", src);
                    println!("{}", toHexString(&buf));
                });
            },
            Err(e) => println!("Could not receive a packet: {}", e)
        }
    }
}
