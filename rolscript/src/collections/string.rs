use core::fmt::{Debug, Display, Formatter, Result as FmtResult, Write};

#[derive(Clone)]
pub struct FixedStrBuf<const S: usize> {
    _len: usize,
    _buf: [u8; S],
}

impl<const S: usize> FixedStrBuf<S> {
    pub fn new() -> Self {
        Self {
            _len: 0,
            _buf: [0; S],
        }
    }

    /// 追加字符串到缓冲区的结尾，返回实际追加的字符串的字节长度。
    /// 保证追加后的缓冲区内的字符串是有效的utf-8字符串。
    pub fn push_str(&mut self, s: &str) -> usize {
        let remaining = S - self._len;

        let mut l = 0;
        for ch in s.chars() {
            let ch_byte_len = ch.len_utf8();
            if l + ch_byte_len > remaining {
                break;
            }
            l += ch_byte_len;
        }
        (&mut self._buf[self._len..self._len + l]).copy_from_slice(&s.as_bytes()[..l]);
        self._len += l;
        l
    }

    pub fn remaining(&self) -> usize {
        S - self._len
    }

    pub fn as_str(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(&self._buf[..self._len]) }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self._buf[..self._len]
    }
}

impl<const S: usize> Write for FixedStrBuf<S> {
    fn write_str(&mut self, s: &str) -> FmtResult {
        self.push_str(s);
        Ok(())
    }
}

impl<const S: usize> core::ops::Deref for FixedStrBuf<S> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl<const S: usize> Display for FixedStrBuf<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str(self.as_str())
    }
}

impl<const S: usize> Debug for FixedStrBuf<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("{:?}", self.as_str()))
    }
}
