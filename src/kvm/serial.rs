/// Collects bytes written by the guest to one serial I/O port.
pub struct SerialConsole {
    port: u16,
    bytes: Vec<u8>,
}

impl SerialConsole {
    /// Creates a collector for one I/O port, for example COM1 at `0x3f8`.
    pub fn new(port: u16) -> Self {
        Self {
            port,
            bytes: Vec::new(),
        }
    }

    /// Records an `OUT` VM-exit when it targets this console's port.
    ///
    /// Returns `true` when the bytes were accepted and `false` for other ports.
    pub fn record_io_out(&mut self, port: u16, data: &[u8]) -> bool {
        if port != self.port {
            return false;
        }

        self.bytes.extend_from_slice(data);
        true
    }

    /// Returns whether any serial output has been collected.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Decodes collected output for display.
    pub fn output_lossy(&self) -> String {
        String::from_utf8_lossy(&self.bytes).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_bytes_for_matching_port() {
        let mut serial = SerialConsole::new(0x3f8);

        assert!(serial.record_io_out(0x3f8, b"Hi"));

        assert_eq!(serial.output_lossy(), "Hi");
    }

    #[test]
    fn ignores_bytes_for_other_ports() {
        let mut serial = SerialConsole::new(0x3f8);

        assert!(!serial.record_io_out(0x2f8, b"ignored"));

        assert!(serial.is_empty());
    }

    #[test]
    fn decodes_invalid_utf8_lossily() {
        let mut serial = SerialConsole::new(0x3f8);

        serial.record_io_out(0x3f8, &[0xff]);

        assert_eq!(serial.output_lossy(), "\u{fffd}");
    }
}
