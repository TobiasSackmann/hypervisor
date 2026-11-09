/// Size of the first guest RAM region in bytes.
pub const GUEST_MEM_SIZE: usize = 0x1000;

/// Physical guest address where the tiny program is loaded and executed.
pub const GUEST_START: u64 = 0x1000;

/// COM1 serial port used by the tiny guest.
pub const SERIAL_PORT_COM1: u16 = 0x3f8;

/// Builds a tiny real-mode-ish x86 guest that writes `message` to COM1 and halts.
///
/// The emitted instruction pattern is intentionally simple:
///
/// ```asm
/// mov dx, 0x3f8
/// mov al, <byte>
/// out dx, al
/// ...
/// hlt
/// ```
pub fn build_serial_message_guest(message: &[u8]) -> Vec<u8> {
    let mut code = Vec::with_capacity(3 + message.len() * 3 + 1);
    let [port_low, port_high] = SERIAL_PORT_COM1.to_le_bytes();

    code.extend_from_slice(&[0xba, port_low, port_high]); // mov dx, SERIAL_PORT_COM1

    for byte in message {
        code.extend_from_slice(&[0xb0, *byte, 0xee]); // mov al, byte; out dx, al
    }

    code.push(0xf4); // hlt
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serial_guest_starts_by_loading_com1_into_dx() {
        let code = build_serial_message_guest(b"A");

        assert_eq!(&code[..3], &[0xba, 0xf8, 0x03]);
    }

    #[test]
    fn serial_guest_writes_each_message_byte() {
        let code = build_serial_message_guest(b"Hi");

        assert_eq!(
            code,
            vec![
                0xba, 0xf8, 0x03, // mov dx, 0x3f8
                0xb0, b'H', 0xee, // mov al, 'H'; out dx, al
                0xb0, b'i', 0xee, // mov al, 'i'; out dx, al
                0xf4, // hlt
            ]
        );
    }

    #[test]
    fn serial_guest_halts_even_without_message() {
        let code = build_serial_message_guest(b"");

        assert_eq!(code, vec![0xba, 0xf8, 0x03, 0xf4]);
    }
}
