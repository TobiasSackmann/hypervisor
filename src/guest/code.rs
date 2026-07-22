pub const GUEST_MEM_SIZE: usize = 0x1000;
pub const GUEST_START: u64 = 0x1000;

// x86 real-mode-ish tiny guest:
//
// mov dx, 0x3f8
// mov al, <byte>
// out dx, al
// ... repeated for every byte in the message
// hlt
pub const CODE: &[u8] = &[
    0xba, 0xf8, 0x03, // mov dx, 0x3f8
    0xb0, b'H', // mov al, 'H'
    0xee, // out dx, al
    0xb0, b'e', // mov al, 'e'
    0xee, // out dx, al
    0xb0, b'l', // mov al, 'l'
    0xee, // out dx, al
    0xb0, b'l', // mov al, 'l'
    0xee, // out dx, al
    0xb0, b'o', // mov al, 'o'
    0xee, // out dx, al
    0xb0, b' ', // mov al, ' '
    0xee, // out dx, al
    0xb0, b'f', // mov al, 'f'
    0xee, // out dx, al
    0xb0, b'r', // mov al, 'r'
    0xee, // out dx, al
    0xb0, b'o', // mov al, 'o'
    0xee, // out dx, al
    0xb0, b'm', // mov al, 'm'
    0xee, // out dx, al
    0xb0, b' ', // mov al, ' '
    0xee, // out dx, al
    0xb0, b'g', // mov al, 'g'
    0xee, // out dx, al
    0xb0, b'u', // mov al, 'u'
    0xee, // out dx, al
    0xb0, b'e', // mov al, 'e'
    0xee, // out dx, al
    0xb0, b's', // mov al, 's'
    0xee, // out dx, al
    0xb0, b't', // mov al, 't'
    0xee, // out dx, al
    0xb0, b'\n', // mov al, '\n'
    0xee,  // out dx, al
    0xf4,  // hlt
];
