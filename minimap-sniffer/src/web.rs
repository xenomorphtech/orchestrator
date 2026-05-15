use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::state::WorldState;

pub type SharedState = Arc<Mutex<WorldState>>;
pub type Clients = Arc<Mutex<Vec<mpsc::Sender<String>>>>;

const INDEX_HTML: &str = include_str!("../web/index.html");

pub fn new_clients() -> Clients {
    Arc::new(Mutex::new(Vec::new()))
}

pub fn spawn_server(bind: SocketAddr, clients: Clients, state: SharedState) -> std::io::Result<()> {
    let listener = TcpListener::bind(bind)?;
    println!("minimap=http://{bind}/");
    thread::spawn(move || {
        for incoming in listener.incoming() {
            match incoming {
                Ok(stream) => {
                    let clients = Arc::clone(&clients);
                    let state = Arc::clone(&state);
                    thread::spawn(move || {
                        if let Err(error) = handle_client(stream, clients, state) {
                            eprintln!("web client error: {error}");
                        }
                    });
                }
                Err(error) => eprintln!("web accept error: {error}"),
            }
        }
    });
    Ok(())
}

pub fn broadcast(clients: &Clients, json: String) {
    let mut clients = clients.lock().expect("clients lock poisoned");
    clients.retain(|client| client.send(json.clone()).is_ok());
}

fn handle_client(
    mut stream: TcpStream,
    clients: Clients,
    state: SharedState,
) -> std::io::Result<()> {
    stream.set_nodelay(true)?;
    let mut buffer = [0u8; 2048];
    let read = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..read]);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");

    match path {
        "/" | "/index.html" => write_response(
            &mut stream,
            "200 OK",
            "text/html; charset=utf-8",
            INDEX_HTML.as_bytes(),
        ),
        "/events" => stream_events(stream, clients, state),
        "/snapshot" => {
            let snapshot = state.lock().expect("state lock poisoned").snapshot_json();
            write_response(
                &mut stream,
                "200 OK",
                "application/json; charset=utf-8",
                snapshot.as_bytes(),
            )
        }
        "/health" => write_response(
            &mut stream,
            "200 OK",
            "application/json; charset=utf-8",
            b"{\"ok\":true}\n",
        ),
        _ => write_response(
            &mut stream,
            "404 Not Found",
            "text/plain; charset=utf-8",
            b"not found\n",
        ),
    }
}

fn stream_events(
    mut stream: TcpStream,
    clients: Clients,
    state: SharedState,
) -> std::io::Result<()> {
    let (tx, rx) = mpsc::channel::<String>();
    clients.lock().expect("clients lock poisoned").push(tx);
    write!(
        stream,
        "HTTP/1.1 200 OK\r\n\
         Content-Type: text/event-stream\r\n\
         Cache-Control: no-cache\r\n\
         Connection: keep-alive\r\n\
         X-Accel-Buffering: no\r\n\
         \r\n"
    )?;

    let snapshot = state.lock().expect("state lock poisoned").snapshot_json();
    write!(stream, "event: snapshot\ndata: {snapshot}\n\n")?;
    stream.flush()?;

    loop {
        match rx.recv_timeout(Duration::from_secs(15)) {
            Ok(json) => {
                write!(stream, "data: {json}\n\n")?;
                stream.flush()?;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                write!(stream, ": keepalive\n\n")?;
                stream.flush()?;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    Ok(())
}

fn write_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    stream.flush()
}
