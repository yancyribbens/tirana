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

use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::Receiver;

use tokio::net::tcp::OwnedReadHalf;
use tokio::net::tcp::OwnedWriteHalf;

use std::env;
use tokio::task::yield_now;

async fn listener(tx: Sender<String>) -> Result<(), Box<dyn Error + Send + Sync>> {
    //println!("starting listener");

    let listener = TcpListener::bind("127.0.0.1:31415").await?;

    loop {
        let (socket, addr) = listener.accept().await?;

        //println!("socet connected: {}", addr);
        let mut framed = BytesCodec::new().framed(socket);

        //println!("{:?}", framed);
        while let Some(message) = framed.next().await {
            match message {
                Ok(_) => {
                    let msg = message.unwrap();
                    let utf8_message = str::from_utf8(&msg).unwrap();
                    let message_string = String::from(utf8_message);
                    println!("{:?}", message_string);

                    if let Err(_) = tx.send(message_string).await {
                        println!("receiver dropped");
                        return Ok(());
                    }
                },
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

async fn irc_writer(mut rx: Receiver<String>, mut writer: OwnedWriteHalf) -> Result<(), Box<dyn Error + Send + Sync>> {
    loop {
        while let Some(val) = rx.recv().await {
            let bytes = val.as_str().as_bytes();
            match writer.try_write(bytes) {
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

    Ok(())
}

async fn irc_reader(tx: Sender<String>, reader: OwnedReadHalf) -> Result<(), Box<dyn Error + Send + Sync>> {
	//println!("start irc reader");
    
	loop {
		reader.readable().await?;

		let mut buf = Vec::with_capacity(4096);

		match reader.try_read_buf(&mut buf) {
			Ok(0) => break,
			Ok(_n) => {
				let response = process_buf(&buf);


				if let Some(response) = response {
                    if let Err(_) = tx.send(response).await {
                        println!("receiver dropped");
                        return Ok(());
                    }
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
	//println!("starting application");

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
	
	//println!("done creating connection string");
    let mut stream: TcpStream = TcpStream::connect("irc.libera.chat:6665").await?;

	stream.write_all(connection.as_bytes()).await?;
	//println!("connection string sent");

    let (read_half, write_half) = stream.into_split();
	//println!("stream created");

    let (tx, rx) = mpsc::channel::<String>(1);
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
