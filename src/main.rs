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

use std::thread;
use std::sync::mpsc::sync_channel;
use std::sync::mpsc::SyncSender;
use std::sync::mpsc::Receiver;

use tokio::net::tcp::OwnedReadHalf;
use tokio::net::tcp::OwnedWriteHalf;

async fn listener(tx: SyncSender<String>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let listener = TcpListener::bind("127.0.0.1:31415").await?;

    loop {
        let (socket, _) = listener.accept().await?;
        let mut framed = BytesCodec::new().framed(socket);

        while let Some(message) = framed.next().await {
            match message {
                Ok(ref bytes) => { tx.send(str::from_utf8(&message.unwrap()).unwrap().to_string()); () },
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

async fn irc_writer(rx: Receiver<String>, mut writer: OwnedWriteHalf) -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("start writer thread");

    loop {
        let val = rx.recv().unwrap();
        println!("read from channel: {}", val);
        writer.write_all(val.as_str().as_bytes());
        println!("done writing from channel to irc");
        match writer.try_write(val.as_str().as_bytes()) {
            Ok(n) => { println!("done writing response"); }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                println!("blocked");
                continue;
            }
            Err(e) => {
                println!("err writing");
                return Err(e.into());
            }
        }
    }

    println!("irc_writer done");

    Ok(())
}

async fn irc_reader(tx: SyncSender<String>, reader: OwnedReadHalf) -> Result<(), Box<dyn Error + Send + Sync>> {
    loop {
        reader.readable().await?;

        let mut buf = Vec::with_capacity(4096);

        match reader.try_read_buf(&mut buf) {
            Ok(0) => break,
            Ok(_n) => {
                let response = process_buf(&buf);

                if let Some(response) = response {
                    let a = response.clone();
                    tx.send(response);
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
    let mut stream: TcpStream = TcpStream::connect("irc.libera.chat:6665").await?;
    stream.write_all(b"NICK test31415\n").await?;
    stream.write_all(b"USER test31415 0 * :Ronnie Reagan\n").await?;

    let (read_half, write_half) = stream.into_split();

    let (tx, rx) = sync_channel::<String>(0);
    let tx0 = tx.clone();

    let listener_handle = tokio::spawn(async move {
        listener(tx).await
    });

    let irc_reader_handle = tokio::spawn(async move {
        irc_reader(tx0, read_half).await
    });

    let irc_writer_handle = tokio::spawn(async move {
        irc_writer(rx, write_half).await
    });

    let (_listener, _irc_reader, _irc_writer) = join!(listener_handle, irc_reader_handle, irc_writer_handle);
    Ok(())
}
