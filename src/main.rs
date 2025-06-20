use std::env;
use std::process::ExitCode;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinSet;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("A TCP echo server\n\nusage: {} [address] [port]", args[0]);
        return ExitCode::FAILURE;
    }

    let listener = TcpListener::bind(format!("{}:{}", args[1], args[2]))
        .await
        .unwrap();

    let mut join_set = JoinSet::new();

    println!("echo server: listening on {}:{}\n", args[1], args[2]);

    loop {
        tokio::select! {
            Ok((mut stream, addr)) = listener.accept() => {
                println!("==> connection established: {addr}");

                join_set.spawn(async move {
                    let mut buf = vec![0u8; 8192];
                    loop {
                        let n = stream.read(&mut buf).await?;
                        if n == 0 {
                            break;
                        }
                        stream.write_all(&buf[0..n]).await?;
                    }

                    println!("==> connection closed: {addr}");
                    Ok::<_, std::io::Error>(())
                });
            }
            Some(res) = join_set.join_next() => {
                if let Err(err) = res {
                    eprintln!("=!> client task failed: {err}");
                }
            }
        }
    }
}
