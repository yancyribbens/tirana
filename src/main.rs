use tokio::net::TcpStream;
use tokio::net::TcpListener;
use tokio::net::tcp::ReadHalf;
use tokio::net::tcp::WriteHalf;

use tokio::io::AsyncWriteExt;
use std::error::Error;
use std::io;
use std::str;

use tokio_stream::StreamExt;
use tokio_util::codec::{BytesCodec, Decoder};
use futures::join;

use irc_proto::command::*;
use irc_proto::message::Message;

async fn handle_inbound(mut stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:31415").await?;

    loop {
        let (socket, _) = listener.accept().await?;
        let mut framed = BytesCodec::new().framed(socket);

        while let Some(message) = framed.next().await {
            match message {
                Ok(bytes) => { stream.write(&bytes).await.unwrap(); println!("bytes: {:?}", bytes) },
                Err(err) => println!("Socket closed with error: {:?}", err),
            }
        }

        println!("Socket received FIN packet and closed connection");
    }
}

async fn irc_stdout(stream: tokio::net::TcpStream) {
    loop {
        stream.readable().await.expect("oops");

        let mut buf = Vec::with_capacity(4096);

        match stream.try_read_buf(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
				//let message = line.parse::<IrcMessage>().unwrap();

				let msg = str::from_utf8(&buf).unwrap();
				let message = msg.parse::<Message>().unwrap();
				println!("read {}", message);
			
				match message.command {
					Command::PING(ref server, ref _server_two) => {
						println!("got ping");
						let cmd = Command::new("PONG", vec![server]).unwrap();
					},

					_ => continue,
				}
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => {
                ()
                //return Err(e.into());
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut stream: TcpStream = TcpStream::connect("irc.libera.chat:6665").await.unwrap();
    //let (mut read_half, mut write_half) = stream.split();
    stream.write(b"NICK test31415\n").await.unwrap();
    stream.write(b"USER test31415 0 * :Ronnie Reagan\n").await.unwrap();
	let h1 = handle_inbound(stream);
    let h2 = irc_stdout(stream);

    //join!(h1, h2);

    Ok(())
}
