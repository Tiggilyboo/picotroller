use core::fmt::Write;

pub struct FmtBuf {
    buf: [u8; 80],
    ptr: usize,
}

impl FmtBuf {
    pub fn new() -> Self {
        Self {
            buf: [0u8; 80],
            ptr: 0,
        }
    }
    pub fn reset(&mut self) {
        self.ptr = 0;
    }

    pub fn str(&self) -> &str {
        core::str::from_utf8(&self.buf[0..self.ptr]).unwrap()
    }

    pub fn bytes(&self) -> &[u8] {
        &self.buf[..self.ptr]
    }

    pub fn buffer(&mut self) -> &mut [u8] {
        &mut self.buf
    }
}

impl Write for FmtBuf {
    fn write_fmt(&mut self, args: core::fmt::Arguments<'_>) -> core::fmt::Result {
        core::fmt::write(self, args)
    }

    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let rest_len = self.buf.len() - self.ptr;
        let len = if rest_len < s.len() {
            rest_len
        } else {
            s.len()
        };

        self.buf[self.ptr..(self.ptr + len)].copy_from_slice(&s.as_bytes()[0..len]);
        self.ptr += len;

        Ok(())
    }
}

