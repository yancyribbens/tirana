use std::time::{SystemTime};
use std::io::Write;

use std::io::prelude::*;
use std::io::BufReader;
use std::net::TcpStream;

//fn prompt(name: &str) -> String {
    //let mut line = String::new();
    //print!("{}", name);
    //std::io::stdout().flush().unwrap();
    //std::io::stdin().read_line(&mut line).expect("Error: Could not read a line");
    //return line.trim().to_string()
//}

fn read_line() -> String {
    let prompt = "> ";
    print!("{}", prompt);

    std::io::stdout().flush().unwrap();

    let mut line = String::new();
    std::io::stdin().read_line(&mut line).expect("Error: Could not read a line");
    return line.trim().to_string()
}

fn main() -> std::io::Result<()> {
    let mut stream = TcpStream::connect("irc.libera.chat:6665")?;

    let nick = "NICK test31415\n".as_bytes();
    let user = "USER test31415 0 * :Ronnie Reagan\n".as_bytes();

    stream.write(nick)?;
    stream.write(user)?;

    let mut reader = BufReader::new(&stream);
    let mut line = String::new();

    loop {
      let len = reader.read_line(&mut line)?;

      if len > 0 {
        println!("{}", line);
      }
    }
    //stream.read(&mut buffer)?;
    //println!("{:?}", buffer);

    Ok(())
}
