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

use std::sync::mpsc::sync_channel;
use std::sync::mpsc::SyncSender;
use std::sync::mpsc::Receiver;

use tokio::net::tcp::OwnedReadHalf;
use tokio::net::tcp::OwnedWriteHalf;

use std::env;

async fn listener(tx: SyncSender<String>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let listener = TcpListener::bind("127.0.0.1:31415").await?;

    loop {
        let (socket, _) = listener.accept().await?;
        let mut framed = BytesCodec::new().framed(socket);

        while let Some(message) = framed.next().await {
            match message {
                Ok(ref _bytes) => { let _res = tx.send(str::from_utf8(&message.unwrap()).unwrap().to_string()); () },
                Err(err) => println!("Socket closed with error: {:?}", err),
            }
        }

        println!("Socket received FIN packet and closed connection");
    }
}

fn process_buf(buf: &Vec<u8>) -> Option<String> {
    let msg_str = str::from_utf8(&buf).unwrap();
    let irc_message = msg_str.parse::<Message>().unwrap();

    let mut response = None;
    match irc_message.command {
        Command::PING(ref server, ref _server_two) => {
            let cmd = Command::new("PONG", vec![server]).unwrap();
            let irc_message = format!(
                "{}\n", Message::from(cmd).to_string());
            
            response = Some(irc_message);
        },
        _ => print!("{}", irc_message),
    }

    response
}

async fn irc_writer(rx: Receiver<String>, mut writer: OwnedWriteHalf) -> Result<(), Box<dyn Error + Send + Sync>> {
    loop {
        let val = rx.recv().unwrap();
        let _res = writer.write_all(val.as_str().as_bytes());
        match writer.try_write(val.as_str().as_bytes()) {
            Ok(_n) => { (); }
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
                    let _res = tx.send(response);
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
	println!("starting application");

	let nick_arg = env::args()
		.nth(1)
		.unwrap();

	let user_arg = env::args()
		.nth(2)
		.unwrap();

	let mode_arg = env::args()
		.nth(3)
		.unwrap();

	let real_arg = env::args()
		.nth(4)
		.unwrap();

	let pass_arg = env::args()
		.nth(5);

	let connection = match pass_arg {
		Some(pass) => format!("PASS {}\nNICK {}\nUSER {} {} * :{}\n", pass, nick_arg, user_arg, mode_arg, real_arg),
		None => format!("NICK {}\nUSER {} {} * :{}\n", nick_arg, user_arg, mode_arg, real_arg),
	};
	
	println!("done creating connection string");
    let mut stream: TcpStream = TcpStream::connect("irc.libera.chat:6665").await?;
	stream.write_all(connection.as_bytes()).await?;

	println!("connection string sent");

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
