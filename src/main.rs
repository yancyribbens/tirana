use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use std::error::Error;
use std::io;
use std::str;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	let mut stream: TcpStream = TcpStream::connect("irc.libera.chat:6665").await.unwrap();
	stream.write(b"NICK test31415\n").await.unwrap();
	stream.write(b"USER test31415 0 * :Ronnie Reagan\n").await.unwrap();

	loop {
		stream.readable().await?;

		let mut buf = Vec::with_capacity(4096);

        match stream.try_read_buf(&mut buf) {
			Ok(0) => break,
			Ok(n) => {
				let msg = str::from_utf8(&buf).unwrap();
				println!("read {}", msg);
			}
			Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
				continue;
			}
			Err(e) => {
				return Err(e.into());
			}
		}
	}

	Ok(())
}
