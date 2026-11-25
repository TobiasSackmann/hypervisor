use std::{
    io::{BufReader, Read, Write},
    os::unix::net::UnixStream,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};

#[derive(Debug)]
pub(super) struct CommandRejected {
    pub command: String,
    pub class: String,
    pub description: String,
}

impl CommandRejected {
    pub fn new(command: String, class: String, description: String) -> Self {
        Self {
            command,
            class,
            description,
        }
    }
}

impl std::fmt::Display for CommandRejected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Command '{}' failed ({}): {}",
            self.command, self.class, self.description
        )
    }
}

impl std::error::Error for CommandRejected {}

/// A small synchronous QMP client, with one request in flight at a time.
pub(super) struct Qmp {
    reader: BufReader<UnixStream>,
    next_id: u64,
}

impl Qmp {
    pub(super) fn new(stream: UnixStream) -> Result<Self> {
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        let mut client = Self {
            reader: BufReader::new(stream),
            next_id: 0,
        };
        let greeting = client.read_message(Instant::now() + Duration::from_secs(5))?;
        ensure!(
            greeting["QMP"].is_object(),
            "invalid QMP greeting: {greeting}"
        );
        client.execute("qmp_capabilities")?;
        Ok(client)
    }

    pub(super) fn execute(&mut self, command: &str) -> Result<Value> {
        self.next_id += 1;
        let id = self.next_id;
        let request = json!({"execute": command, "id": id});
        writeln!(self.reader.get_mut(), "{request}\r").context("write QMP command")?;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let response = self.read_message(deadline)?;
            if response.get("event").is_some() {
                // Event handling will be added with persistent lifecycle monitoring.
                continue;
            }
            if response.get("id") != Some(&json!(id)) {
                if response.get("id").is_none() && response.get("error").is_some() {
                    bail!("QMP protocol error: {}", response["error"]);
                }
                continue;
            }
            if let Some(error) = response.get("error") {
                bail!(CommandRejected::new(
                    command.to_owned(),
                    error["class"]
                        .as_str()
                        .context("QMP error missing class")?
                        .to_owned(),
                    error["desc"]
                        .as_str()
                        .context("QMP error missing description")?
                        .to_owned(),
                ));
            }

            return response
                .get("return")
                .cloned()
                .context("QMP response missing return value");
        }
    }

    fn read_message(&mut self, deadline: Instant) -> Result<Value> {
        let mut bytes = Vec::new();
        // Bound both the message size and the total wait, including partial lines.
        loop {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .filter(|duration| !duration.is_zero())
                .context("QMP response timed out")?;
            self.reader.get_ref().set_read_timeout(Some(remaining))?;
            let mut byte = [0];
            self.reader
                .read_exact(&mut byte)
                .context("read QMP response")?;
            bytes.push(byte[0]);
            ensure!(bytes.len() <= 1024 * 1024, "QMP message exceeds 1 MiB");
            if byte[0] == b'\n' {
                return serde_json::from_slice(&bytes).context("decode QMP JSON");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::BufRead, thread};

    fn mock(error: bool) -> (UnixStream, thread::JoinHandle<()>) {
        let (client, server) = UnixStream::pair().unwrap();
        let task = thread::spawn(move || {
            server
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut server = BufReader::new(server);
            writeln!(server.get_mut(), "{}", json!({"QMP": {}})).unwrap();
            for command in ["qmp_capabilities", "query-status"] {
                let mut line = String::new();
                server.read_line(&mut line).unwrap();
                let request: Value = serde_json::from_str(&line).unwrap();
                assert_eq!(request["execute"], command);
                let response = if command == "query-status" && error {
                    json!({"error": {"class": "GenericError", "desc": "test failure"}, "id": request["id"]})
                } else {
                    json!({"return": {"status": "running"}, "id": request["id"]})
                };
                writeln!(server.get_mut(), "{}", json!({"event": "RESUME"})).unwrap();
                writeln!(server.get_mut(), "{response}").unwrap();
            }
        });
        (client, task)
    }

    #[test]
    fn negotiates_and_matches_responses_around_events() {
        let (stream, task) = mock(false);
        let mut client = Qmp::new(stream).unwrap();
        assert_eq!(client.execute("query-status").unwrap()["status"], "running");
        task.join().unwrap();
    }

    #[test]
    fn reports_command_errors() {
        let (stream, task) = mock(true);
        let mut client = Qmp::new(stream).unwrap();
        assert!(
            client
                .execute("query-status")
                .unwrap_err()
                .to_string()
                .contains("test failure")
        );
        task.join().unwrap();
    }

    #[test]
    fn rejects_eof_and_invalid_greetings() {
        let (client, mut server) = UnixStream::pair().unwrap();
        writeln!(server, "{{}}").unwrap();
        assert!(Qmp::new(client).is_err());
        let (client, server) = UnixStream::pair().unwrap();
        drop(server);
        assert!(Qmp::new(client).is_err());
    }
}
