// ============================================================================
// codec.rs — dependency-free deterministic serialization and hashing (T01 C05)
//
// The contract requires a deterministic byte encoding: equal observable state
// must encode to equal bytes on every run, with no dependence on hash-map
// iteration order or addresses. `Writer` provides length-prefixed primitives
// and a compact, well-defined layout. `sha256` provides a real content hash
// for snapshots and the frozen contract artifact.
// ============================================================================

/// A little-endian, length-prefixed canonical byte writer.
#[derive(Clone, Debug, Default)]
pub struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    /// Create an empty writer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one byte.
    pub fn u8(&mut self, value: u8) {
        self.buf.push(value);
    }

    /// Append a boolean (`0`/`1`).
    pub fn bool(&mut self, value: bool) {
        self.u8(u8::from(value));
    }

    /// Append a little-endian `u16`.
    pub fn u16(&mut self, value: u16) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    /// Append a little-endian `u32`.
    pub fn u32(&mut self, value: u32) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    /// Append a little-endian `u64`.
    pub fn u64(&mut self, value: u64) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    /// Append a little-endian `i64`.
    pub fn i64(&mut self, value: i64) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    /// Append a little-endian `i128`.
    pub fn i128(&mut self, value: i128) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    /// Append a length-prefixed byte string.
    pub fn bytes(&mut self, value: &[u8]) {
        self.u64(value.len() as u64);
        self.buf.extend_from_slice(value);
    }

    /// Append a length-prefixed UTF-8 string.
    pub fn str(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    /// Append raw bytes with no length prefix.
    pub fn raw(&mut self, value: &[u8]) {
        self.buf.extend_from_slice(value);
    }

    /// Consume the writer and return the bytes.
    pub fn finish(self) -> Vec<u8> {
        self.buf
    }
}

/// SHA-256 of `data`, as specified by FIPS 180-4.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut message = data.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());

    // SHA-256 requires the padded message length to be a multiple of 64: the
    // `0x80` terminator, zero fill, and 8-byte length below guarantee it.
    // `as_chunks` therefore yields no remainder and keeps the block layout
    // identical to the previous `chunks_exact(64)` loop.
    let (blocks, remainder) = message.as_chunks::<64>();
    debug_assert!(
        remainder.is_empty(),
        "SHA-256 input must be block-aligned after padding"
    );

    for chunk in blocks {
        let mut w = [0u32; 64];
        for (index, word) in w.iter_mut().take(16).enumerate() {
            let start = index * 4;
            *word = u32::from_be_bytes([
                chunk[start],
                chunk[start + 1],
                chunk[start + 2],
                chunk[start + 3],
            ]);
        }
        for index in 16..64 {
            let s0 = w[index - 15].rotate_right(7)
                ^ w[index - 15].rotate_right(18)
                ^ (w[index - 15] >> 3);
            let s1 = w[index - 2].rotate_right(17)
                ^ w[index - 2].rotate_right(19)
                ^ (w[index - 2] >> 10);
            w[index] = w[index - 16]
                .wrapping_add(s0)
                .wrapping_add(w[index - 7])
                .wrapping_add(s1);
        }

        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];

        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[index])
                .wrapping_add(w[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }

    let mut out = [0u8; 32];
    for (index, word) in state.iter().enumerate() {
        out[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// Lowercase hexadecimal encoding of a 32-byte hash.
pub fn hex32(hash: &[u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in hash {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Structured decode failure.
///
/// Every decoder built on [`Reader`] returns `Result<_, CodecError>`; no
/// decoder panics on malformed input, per the crate's checked-errors-only
/// guarantee (`todo!()`/`unimplemented!()`/`unwrap()` are banned in decoders).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodecError {
    /// The input ended before the value was complete.
    Truncated,
    /// Bytes remain after the value was fully read.
    TrailingBytes,
    /// A length-prefixed string is not valid UTF-8.
    InvalidUtf8,
    /// An unknown tag or discriminant byte. Carries the offending tag.
    InvalidTag(u8),
    /// A recognized tag whose record shape is reserved for a future contract
    /// revision and has no frozen encoding yet. Carries a static context note.
    /// Returned instead of panicking for pending `/6` shapes.
    Unsupported(&'static str),
}

impl std::fmt::Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated => write!(f, "truncated input"),
            Self::TrailingBytes => write!(f, "trailing bytes after value"),
            Self::InvalidUtf8 => write!(f, "invalid UTF-8 in length-prefixed string"),
            Self::InvalidTag(tag) => write!(f, "unknown tag {tag}"),
            Self::Unsupported(note) => write!(f, "unsupported shape: {note}"),
        }
    }
}

impl std::error::Error for CodecError {}

/// A checked little-endian reader: the fallible mirror of [`Writer`].
///
/// Every read is bounds-checked and returns [`CodecError`] on short input, so
/// malformed snapshots fail as structured errors rather than panics. The
/// primitive order and layout exactly mirror [`Writer`] (`u64` length prefixes
/// for `bytes`/`str`, fixed little-endian integers); this type only adds the
/// read direction and changes no existing byte format.
#[derive(Clone, Copy, Debug)]
pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    /// Borrow `bytes` as a readable cursor positioned at the start.
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { buf: bytes, pos: 0 }
    }

    /// Number of bytes not yet consumed.
    pub fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }

    /// Whether every byte was consumed.
    pub fn is_empty(&self) -> bool {
        self.pos >= self.buf.len()
    }

    /// Current cursor offset.
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Take exactly `len` bytes or fail with [`CodecError::Truncated`].
    pub fn raw(&mut self, len: usize) -> Result<&'a [u8], CodecError> {
        let end = self.pos.saturating_add(len);
        let slice = self.buf.get(self.pos..end).ok_or(CodecError::Truncated)?;
        self.pos = end;
        Ok(slice)
    }

    /// Read one byte.
    pub fn u8(&mut self) -> Result<u8, CodecError> {
        let byte = *self.buf.get(self.pos).ok_or(CodecError::Truncated)?;
        self.pos = self.pos.saturating_add(1);
        Ok(byte)
    }

    /// Read a boolean encoded as `0`/`1`; any other byte is [`CodecError::InvalidTag`].
    pub fn bool(&mut self) -> Result<bool, CodecError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            tag => Err(CodecError::InvalidTag(tag)),
        }
    }

    /// Read a little-endian `u16`.
    pub fn u16(&mut self) -> Result<u16, CodecError> {
        let bytes: [u8; 2] = self
            .raw(2)?
            .try_into()
            .map_err(|_| CodecError::Truncated)?;
        Ok(u16::from_le_bytes(bytes))
    }

    /// Read a little-endian `u32`.
    pub fn u32(&mut self) -> Result<u32, CodecError> {
        let bytes: [u8; 4] = self
            .raw(4)?
            .try_into()
            .map_err(|_| CodecError::Truncated)?;
        Ok(u32::from_le_bytes(bytes))
    }

    /// Read a little-endian `u64`.
    pub fn u64(&mut self) -> Result<u64, CodecError> {
        let bytes: [u8; 8] = self
            .raw(8)?
            .try_into()
            .map_err(|_| CodecError::Truncated)?;
        Ok(u64::from_le_bytes(bytes))
    }

    /// Read a little-endian `i64`.
    pub fn i64(&mut self) -> Result<i64, CodecError> {
        let bytes: [u8; 8] = self
            .raw(8)?
            .try_into()
            .map_err(|_| CodecError::Truncated)?;
        Ok(i64::from_le_bytes(bytes))
    }

    /// Read a little-endian `i128`.
    pub fn i128(&mut self) -> Result<i128, CodecError> {
        let bytes: [u8; 16] = self
            .raw(16)?
            .try_into()
            .map_err(|_| CodecError::Truncated)?;
        Ok(i128::from_le_bytes(bytes))
    }

    /// Read a length-prefixed byte string (mirrors [`Writer::bytes`]).
    pub fn bytes(&mut self) -> Result<Vec<u8>, CodecError> {
        let len = usize::try_from(self.u64()?).map_err(|_| CodecError::Truncated)?;
        Ok(self.raw(len)?.to_vec())
    }

    /// Read a length-prefixed UTF-8 string (mirrors [`Writer::str`]).
    pub fn string(&mut self) -> Result<String, CodecError> {
        let bytes = self.bytes()?;
        String::from_utf8(bytes).map_err(|_| CodecError::InvalidUtf8)
    }

    /// Fail with [`CodecError::TrailingBytes`] unless every byte was consumed.
    pub fn finish(&self) -> Result<(), CodecError> {
        if self.pos == self.buf.len() {
            Ok(())
        } else {
            Err(CodecError::TrailingBytes)
        }
    }
}
