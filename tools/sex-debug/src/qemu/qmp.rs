use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use serde_json::Value;

pub struct QmpClient {
    stream: UnixStream,
}

impl QmpClient {
    pub fn connect(socket_path: &str) -> std::io::Result<Self> {
        let stream = UnixStream::connect(socket_path)?;
        let mut client = Self { stream };
        client.negotiate()?;
        Ok(client)
    }

    fn negotiate(&mut self) -> std::io::Result<()> {
        let mut reader = BufReader::new(self.stream.try_clone()?);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        
        let cmd = serde_json::json!({ "execute": "qmp_capabilities" });
        let cmd_str = serde_json::to_string(&cmd)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))? + "\n";
        self.stream.write_all(cmd_str.as_bytes())?;
        
        line.clear();
        reader.read_line(&mut line)?;
        Ok(())
    }

    pub fn info_registers(&mut self) -> std::io::Result<Option<(u64, u32, u64)>> {
        let cmd = serde_json::json!({
            "execute": "human-monitor-command",
            "arguments": {
                "command-line": "info registers"
            }
        });
        
        let cmd_str = serde_json::to_string(&cmd)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))? + "\n";
        self.stream.write_all(cmd_str.as_bytes())?;

        let mut reader = BufReader::new(self.stream.try_clone()?);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        
        let resp: Value = serde_json::from_str(&line)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        
        if let Some(return_val) = resp.get("return").and_then(|v| v.as_str()) {
            let mut rip = 0;
            let mut pkru = 0;
            let mut cr3 = 0;

            for l in return_val.lines() {
                if l.starts_with("RIP=") {
                    if let Some(val) = l.split_whitespace().next() {
                        let hex = val.trim_start_matches("RIP=");
                        rip = u64::from_str_radix(hex, 16).unwrap_or(0);
                    }
                }
                if l.starts_with("PKRU=") {
                    if let Some(val) = l.split_whitespace().next() {
                        let hex = val.trim_start_matches("PKRU=");
                        pkru = u32::from_str_radix(hex, 16).unwrap_or(0);
                    }
                }
                if l.starts_with("CR3=") {
                    if let Some(val) = l.split_whitespace().next() {
                        let hex = val.trim_start_matches("CR3=");
                        cr3 = u64::from_str_radix(hex, 16).unwrap_or(0);
                    }
                }
            }
            Ok(Some((rip, pkru, cr3)))
        } else {
            Ok(None)
        }
    }
}