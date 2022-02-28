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

type SharedMessages = Arc<Mutex<Vec<String>>>;

async fn listener() -> Result<(), Box<dyn Error + Send + Sync>> {

    let listener = TcpListener::bind("127.0.0.1:31415").await?;

    loop {
        //let messages = shared_messages.lock().unwrap();
        //println!("messages length: {}", messages.len());

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

fn process_buf(buf: &Vec<u8>) {
    let msg_str = str::from_utf8(&buf).unwrap();
    let irc_message = msg_str.parse::<Message>().unwrap();
    println!("{}", irc_message);

    match irc_message.command {
        Command::PING(ref server, ref _server_two) => {
            println!("got a ping");
            let cmd = Command::new("PONG", vec![server]).unwrap();
            let irc_message = format!(
                "{}\n", Message::from(cmd).to_string());
            //let mut shared_messages = shared_messages.lock().unwrap();
            //shared_messages.push(irc_message);
        },
        _ => println!("{}", irc_message),
    }

    println!("{}", irc_message);
}

async fn irc_stdout() -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut stream: TcpStream = TcpStream::connect("irc.libera.chat:6665").await?;

    stream.write_all(b"NICK test31415\n").await?;
    stream.write_all(b"USER test31415 0 * :Ronnie Reagan\n").await?;

    loop {
        stream.readable().await?;

        let mut buf = Vec::with_capacity(4096);

        match stream.try_read_buf(&mut buf) {
            Ok(0) => break,
            Ok(n) => process_buf(&buf),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => {
                ()
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    //let shared_messages = Arc::new(Mutex::new(Vec::new()));
    //let shared_messages0 = shared_messages.clone();

    let h1 = tokio::spawn(async move {
        listener().await
    });

    let h2 = tokio::spawn(async move {
        irc_stdout().await
    });


    //let h1 = handle_inbound(write_half, shared_messages);
    //let h2 = irc_stdout(read_half, shared_messages0);

    join!(h1);

    Ok(())
}
