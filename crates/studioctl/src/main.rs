use anyhow::{Context, Result, bail};
use serde_json::json;
use std::env;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use studio_core::socket_path;

fn main() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let command = args.first().map(String::as_str).unwrap_or("status");
    let request = match command {
        "status" | "watch" => json!({ "command": command }),
        "command" if args.len() >= 3 => {
            let payload = args
                .get(3)
                .map(|text| serde_json::from_str(text).context("parse command payload"))
                .transpose()?
                .unwrap_or_else(|| json!({}));
            json!({
                "command": "device_command",
                "deviceId": args[1],
                "action": args[2],
                "payload": payload
            })
        }
        _ => bail!("usage: studioctl [status|watch|command <device-id> <action> [payload-json]]"),
    };

    let path = socket_path();
    let mut stream = UnixStream::connect(&path)
        .with_context(|| format!("connect to Studio Control at {}", path.display()))?;
    serde_json::to_writer(&mut stream, &request)?;
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
        if command != "watch" {
            break;
        }
    }
    Ok(())
}
