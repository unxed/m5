//! Scanning of CSI sequences (`ESC [ params final`) and of string sequences (APC).

/// A parsed CSI sequence.
pub(crate) struct Csi {
    /// Private-parameter marker (`<`, `=`, `>` or `?`) that opens the parameters, if any.
    pub private: Option<u8>,
    /// Parameters split at `;`, each split further at `:`. Empty fields are `None`.
    pub params: Vec<Vec<Option<u32>>>,
    /// Whether intermediate bytes (0x20..=0x2F) were present.
    pub has_intermediate: bool,
    /// The final byte (0x40..=0x7E).
    pub final_byte: u8,
}

impl Csi {
    /// The first (main) value of parameter `i`.
    pub fn param(&self, i: usize) -> Option<u32> {
        self.sub(i, 0)
    }

    /// Sub-parameter `j` (after `:`) of parameter `i`.
    pub fn sub(&self, i: usize, j: usize) -> Option<u32> {
        self.params.get(i)?.get(j).copied().flatten()
    }
}

/// Outcome of scanning for a CSI sequence.
pub(crate) enum Scan {
    /// More bytes are needed.
    Incomplete,
    /// Malformed; this many bytes (always at least 2) are to be skipped.
    Bad(usize),
    /// A complete sequence and its length in bytes.
    Seq(Csi, usize),
}

/// Scans the CSI sequence at the start of `buf`, which begins with `ESC [`.
pub(crate) fn scan(buf: &[u8]) -> Scan {
    let mut has_intermediate = false;
    for (i, &b) in buf.iter().enumerate().skip(2) {
        match b {
            0x30..=0x3F if !has_intermediate => {}
            0x20..=0x2F => has_intermediate = true,
            0x40..=0x7E => {
                let seq = build(&buf[2..i], has_intermediate, b);
                return Scan::Seq(seq, i + 1);
            }
            _ => return Scan::Bad(i),
        }
    }
    Scan::Incomplete
}

fn build(body: &[u8], has_intermediate: bool, final_byte: u8) -> Csi {
    let (private, body) = match body.first() {
        Some(&b) if matches!(b, b'<' | b'=' | b'>' | b'?') => (Some(b), &body[1..]),
        _ => (None, body),
    };
    let params = body
        .split(|&b| b == b';')
        .map(|field| field.split(|&b| b == b':').map(parse_number).collect())
        .collect();
    Csi {
        private,
        params,
        has_intermediate,
        final_byte,
    }
}

/// Decimal number of the digits in `s` (other bytes are ignored); `None` if there are none.
fn parse_number(s: &[u8]) -> Option<u32> {
    let mut value: Option<u32> = None;
    for &b in s.iter().filter(|b| b.is_ascii_digit()) {
        let digit = u32::from(b - b'0');
        value = Some(value.unwrap_or(0).saturating_mul(10).saturating_add(digit));
    }
    value
}

/// Outcome of scanning for a string sequence such as APC (`ESC _ ... BEL` or `ESC _ ... ESC \`).
pub(crate) enum StrScan {
    /// The terminator has not arrived yet.
    Incomplete,
    /// An `ESC` that does not start `ST` interrupted the string; skip this many bytes.
    Aborted(usize),
    /// The whole sequence, terminator included, is this many bytes long.
    Done(usize),
}

/// Scans the string sequence at the start of `buf`, which begins with `ESC` and an introducer.
pub(crate) fn scan_string(buf: &[u8]) -> StrScan {
    for (i, &b) in buf.iter().enumerate().skip(2) {
        match b {
            0x07 => return StrScan::Done(i + 1),
            0x1B => {
                return match buf.get(i + 1) {
                    None => StrScan::Incomplete,
                    Some(&b'\\') => StrScan::Done(i + 2),
                    Some(_) => StrScan::Aborted(i),
                };
            }
            _ => {}
        }
    }
    StrScan::Incomplete
}
