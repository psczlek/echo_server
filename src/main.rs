use colored::Colorize;
use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::process::ExitCode;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, ToSocketAddrs, UdpSocket};
use tokio::signal;
use tokio::task::JoinSet;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        args.extend_from_slice(&["127.0.0.1".to_owned(), "12345".to_owned(), "tcp".to_owned()]);
    } else if args.len() == 2 {
        println!(
            "A TCP/UDP echo server\n\n{} {} {}",
            "Usage:".green().bold(),
            args[0].cyan().bold(),
            "[address] [port] Optional([tcp|udp])".cyan(),
        );
        return ExitCode::FAILURE;
    } else if args.len() == 3 {
        args.push("tcp".to_owned());
    }

    let (addr, proto) = (format!("{}:{}", &args[1], &args[2]), args[3].as_str());

    println!(
        "{} {}\n",
        "[+]".yellow().bold(),
        format!("Listening on {} ({})", addr, proto.to_uppercase())
            .green()
            .bold()
    );

    match proto {
        "tcp" => match tcp_echo(addr).await {
            Ok(_) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{}: {}", "error".red().bold(), e);
                ExitCode::FAILURE
            }
        },
        "udp" => match udp_echo(addr).await {
            Ok(_) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{}: {}", "error".red().bold(), e);
                return ExitCode::FAILURE;
            }
        },
        _ => {
            eprintln!(
                "{}: Invalid protocol: {} [tcp, udp]",
                "error".red().bold(),
                proto,
            );
            ExitCode::FAILURE
        }
    }
}

async fn tcp_echo<A: ToSocketAddrs>(addr: A) -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(addr).await?;
    let mut join_set = JoinSet::new();

    loop {
        tokio::select! {
            Ok((mut stream, addr)) = listener.accept() => {
                println!(
                    "{} Connection established: {}",
                    "==>".blue().bold(),
                    addr,
                );

                join_set.spawn(async move {
                    let mut buf = vec![0u8; 16384];
                    loop {
                        let n = stream.read(&mut buf).await?;
                        if n == 0 {
                            break;
                        }
                        stream.write_all(&buf[..n]).await?;
                    }

                    println!(
                        "{} Connection closed: {}",
                        "==>".yellow().bold(),
                        addr,
                    );
                    Ok::<_, std::io::Error>(())
                });
            },
            Some(res) = join_set.join_next() => {
                if let Err(err) = res {
                    eprintln!("{} Client task failed: {err}", "=!>".red().bold());
                }
            },
            _ = signal::ctrl_c() => {
                join_set.shutdown().await;
                break;
            },
        }
    }

    Ok(())
}

async fn udp_echo<A: ToSocketAddrs>(addr: A) -> Result<(), Box<dyn Error>> {
    let socket = UdpSocket::bind(addr).await?;
    let mut buf = vec![0u8; 16384];
    let mut clients_seen = HashSet::new();

    loop {
        tokio::select! {
            Ok((len, addr)) = socket.recv_from(&mut buf) => {
                if !clients_seen.contains(&addr) {
                    println!(
                        "{} Got data from: {}",
                        "==>".blue().bold(),
                        addr,
                    );
                }

                clients_seen.insert(addr);
                socket.send_to(&buf[..len], addr).await?;
            },
            _ = signal::ctrl_c() => break,
        }
    }

    // Notify each client seen that we're dead.
    for addr in clients_seen {
        socket.send_to(&[0xde, 0xad], addr).await?;
    }

    Ok(())
}
