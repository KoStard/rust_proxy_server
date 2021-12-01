use std::{net::{TcpListener, TcpStream}, io::{Read, Write}};

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:80")?;

    // accept connections and process them serially
    for stream in listener.incoming() {
        handle_client(stream?)?;
    }
    Ok(())
}

fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    println!("{:?}", stream);
    stream.write("HELLO".as_bytes())?;
    println!("Connection established");
    loop {
        let mut buffer = [0; 500];
        let count = stream.read(&mut buffer)?;
        let s = String::from_utf8_lossy(&buffer);
        stream.write(&buffer)?;
        println!("{} {}", s, count);
        if count == 0 {
            println!("Closing");
            break;
        }
    }
    Ok(())
}
