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

use bytes::Bytes;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use irc_proto::{
    message::Message,
    command::*,
};

type conduit = Arc<Mutex<HashMap<String, Bytes>>>;

async fn handle_inbound(mut write_half: WriteHalf<'_>) -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:31415").await?;

    loop {
        let (socket, _) = listener.accept().await?;
        let mut framed = BytesCodec::new().framed(socket);

        while let Some(message) = framed.next().await {
            match message {
                Ok(bytes) => { write_half.write(&bytes).await.unwrap(); println!("bytes: {:?}", bytes) },
                Err(err) => println!("Socket closed with error: {:?}", err),
            }
        }

        println!("Socket received FIN packet and closed connection");
    }
}

fn process_buf(buf: &Vec<u8>) {
	let msg_str = str::from_utf8(&buf).unwrap();
	let irc_message = msg_str.parse::<Message>().unwrap();
	match irc_message.command {
		Command::PING(ref server, ref _server_two) => {
			println!("PING: {}", server);
			//let cmd = Command::new("PONG", vec![server]).unwrap();
			//let irc_message = format!(
				//"{}\n", IrcMessage::from(cmd).to_string());
		},
		_ => println!("{}", irc_message),
	}

	println!("{}", irc_message);
}

async fn irc_stdout(read_half: ReadHalf<'_>) {
    loop {
        read_half.readable().await.expect("oops");

        let mut buf = Vec::with_capacity(4096);

        match read_half.try_read_buf(&mut buf) {
            Ok(0) => break,
            Ok(n) => process_buf(&buf),
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
    let (mut read_half, mut write_half) = stream.split();
    write_half.write(b"NICK test31415\n").await.unwrap();
    write_half.write(b"USER test31415 0 * :Ronnie Reagan\n").await.unwrap();
    let h1 = handle_inbound(write_half);
    let h2 = irc_stdout(read_half);

    join!(h1, h2);

    Ok(())
}
