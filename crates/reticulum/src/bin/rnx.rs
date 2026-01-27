use clap::Parser;
use reticulum::e2e_harness::{
    build_http_post, build_receive_params, build_rpc_frame, build_send_params, build_daemon_args,
    is_ready_line, message_present, parse_http_response_body, parse_rpc_frame,
    simulated_delivery_notice, timestamp_millis, Cli, Command,
};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command as ProcessCommand, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

fn main() {
    let cli = Cli::parse();
    if let Err(err) = run(cli) {
        eprintln!("rnx error: {}", err);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> io::Result<()> {
    match cli.command {
        Command::E2e {
            a_port,
            b_port,
            timeout_secs,
            keep,
        } => run_e2e(a_port, b_port, timeout_secs, keep),
    }
}

fn run_e2e(a_port: u16, b_port: u16, timeout_secs: u64, keep: bool) -> io::Result<()> {
    let timeout = Duration::from_secs(timeout_secs);
    let a_rpc = format!("127.0.0.1:{}", a_port);
    let b_rpc = format!("127.0.0.1:{}", b_port);

    let a_dir = tempfile::TempDir::new()?;
    let b_dir = tempfile::TempDir::new()?;
    let a_db = a_dir.path().join("reticulum.db");
    let b_db = b_dir.path().join("reticulum.db");

    let mut a_child = spawn_daemon(&a_rpc, &a_db)?;
    wait_for_ready(
        a_child
            .stdout
            .take()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "missing daemon stdout"))?,
        timeout,
    )?;

    let mut b_child = spawn_daemon(&b_rpc, &b_db)?;
    wait_for_ready(
        b_child
            .stdout
            .take()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "missing daemon stdout"))?,
        timeout,
    )?;

    println!("{}", simulated_delivery_notice(&a_rpc, &b_rpc));

    let message_id = format!("e2e-{}", timestamp_millis());
    let content = "hello from rnx e2e";

    let mut req_id = 1u64;
    let send_params = build_send_params(&message_id, "peer-a", "peer-b", content);
    rpc_call(&a_rpc, req_id, "send_message", Some(send_params))?;
    req_id += 1;

    let receive_params = build_receive_params(&message_id, "peer-a", "peer-b", content);
    rpc_call(&b_rpc, req_id, "receive_message", Some(receive_params))?;
    req_id += 1;

    let found = poll_for_message(&b_rpc, &message_id, timeout, req_id)?;
    if !found {
        cleanup_child(&mut a_child, keep);
        cleanup_child(&mut b_child, keep);
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "message not delivered",
        ));
    }

    cleanup_child(&mut a_child, keep);
    cleanup_child(&mut b_child, keep);
    println!("E2E ok: message {} delivered", message_id);
    Ok(())
}

fn spawn_daemon(rpc: &str, db_path: &PathBuf) -> io::Result<Child> {
    let mut cmd = ProcessCommand::new(reticulumd_path()?);
    cmd.args(build_daemon_args(rpc, &db_path.to_string_lossy(), 0));
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::inherit());
    cmd.spawn()
}

fn reticulumd_path() -> io::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let dir = exe
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "missing exe parent"))?;
    let candidate = dir.join("reticulumd");
    if candidate.exists() {
        Ok(candidate)
    } else {
        Ok(PathBuf::from("reticulumd"))
    }
}

fn wait_for_ready<R: Read + Send + 'static>(reader: R, timeout: Duration) -> io::Result<()> {
    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        let mut lines = BufReader::new(reader).lines();
        while let Some(Ok(line)) = lines.next() {
            let _ = tx.send(line);
        }
    });

    let deadline = Instant::now() + timeout;
    loop {
        let now = Instant::now();
        if now >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "daemon did not become ready",
            ));
        }
        let remaining = deadline.saturating_duration_since(now);
        match rx.recv_timeout(remaining) {
            Ok(line) => {
                if is_ready_line(&line) {
                    return Ok(());
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "daemon stdout closed",
                ));
            }
        }
    }
}

fn rpc_call(
    rpc: &str,
    id: u64,
    method: &str,
    params: Option<serde_json::Value>,
) -> io::Result<reticulum::rpc::RpcResponse> {
    let frame = build_rpc_frame(id, method, params)?;
    let request = build_http_post("/rpc", rpc, &frame);
    let mut stream = TcpStream::connect(rpc)?;
    stream.write_all(&request)?;
    stream.shutdown(Shutdown::Write)?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    let body = parse_http_response_body(&response)?;
    parse_rpc_frame(&body)
}

fn poll_for_message(
    rpc: &str,
    message_id: &str,
    timeout: Duration,
    mut request_id: u64,
) -> io::Result<bool> {
    let deadline = Instant::now() + timeout;
    loop {
        let response = rpc_call(rpc, request_id, "list_messages", None)?;
        request_id = request_id.wrapping_add(1);
        if message_present(&response, message_id) {
            return Ok(true);
        }
        if Instant::now() >= deadline {
            return Ok(false);
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn cleanup_child(child: &mut Child, keep: bool) {
    if keep {
        return;
    }
    let _ = child.kill();
    let _ = child.wait();
}
