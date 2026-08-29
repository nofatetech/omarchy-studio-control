use anyhow::{Context, Result, bail};
use serde_json::json;
use std::env;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use studio_core::socket_path;

fn main() -> Result<()> {
    let command = env::args().nth(1).unwrap_or_else(|| "status".to_string());
    if command != "status" && command != "watch" {
        bail!("usage: studioctl [status|watch]");
    }

    let path = socket_path();
    let mut stream = UnixStream::connect(&path)
        .with_context(|| format!("connect to Studio Control at {}", path.display()))?;
    serde_json::to_writer(&mut stream, &json!({ "command": command }))?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let mut line = String::new();

    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        output.write_all(line.as_bytes())?;
        output.flush()?;
        if command == "status" {
            break;
        }
    }
    Ok(())
}
