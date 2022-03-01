use tokio::net::TcpStream;
use tokio::net::TcpListener;

use tokio::io::AsyncWriteExt;
use std::error::Error;
use std::io;
use std::str;

use tokio_stream::StreamExt;
use tokio_util::codec::{BytesCodec, Decoder};
use futures::join;

use irc_proto::{
    message::Message,
    command::*,
};

async fn listener() -> Result<(), Box<dyn Error + Send + Sync>> {

    let listener = TcpListener::bind("127.0.0.1:31415").await?;

    loop {
        let (socket, _) = listener.accept().await?;
        let mut framed = BytesCodec::new().framed(socket);

        while let Some(message) = framed.next().await {
            match message {
                Ok(bytes) => { println!("bytes: {:?}", bytes) },
                Err(err) => println!("Socket closed with error: {:?}", err),
            }
        }

        println!("Socket received FIN packet and closed connection");
    }
}

fn process_buf(buf: &Vec<u8>) -> Option<String> {
    let msg_str = str::from_utf8(&buf).unwrap();
    let irc_message = msg_str.parse::<Message>().unwrap();
    print!("{}", irc_message);

    let mut response = None;
    match irc_message.command {
        Command::PING(ref server, ref _server_two) => {
            println!("got a ping");
            let cmd = Command::new("PONG", vec![server]).unwrap();
            let irc_message = format!(
                "{}\n", Message::from(cmd).to_string());
            
            response = Some(irc_message);
        },
        _ => (),
    }

    response
}

async fn irc() -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut stream: TcpStream = TcpStream::connect("irc.libera.chat:6665").await?;

    stream.write_all(b"NICK test31415\n").await?;
    stream.write_all(b"USER test31415 0 * :Ronnie Reagan\n").await?;

    let (read_half, write_half) = stream.split();

    loop {
        read_half.readable().await?;

        let mut buf = Vec::with_capacity(4096);

        match read_half.try_read_buf(&mut buf) {
            Ok(0) => break,
            Ok(_n) => {
                let response = process_buf(&buf);

                if let Some(response) = response {
                    println!("writing pong");
                    write_half.try_write(response.as_bytes())?;
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(_e) => {
                ()
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let h1 = tokio::spawn(async move {
        listener().await
    });

    let h2 = tokio::spawn(async move {
        irc().await
    });

    let (_listener, _irc) = join!(h1, h2);
    Ok(())
}
