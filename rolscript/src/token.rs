#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pos {
    pub byte_pos: usize,
    pub char_pos: usize,
    pub line: usize,
    pub column: usize,
}

impl Pos {
    pub fn new(byte_pos: usize, char_pos: usize, line: usize, column: usize) -> Self {
        Self {
            byte_pos,
            char_pos,
            line,
            column,
        }
    }
}

impl Default for Pos {
    fn default() -> Self {
        Self {
            byte_pos: 0,
            char_pos: 0,
            line: 1,
            column: 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Len {
    pub byte_len: usize,
    pub char_len: usize,
}

impl Len {
    pub fn new(byte_len: usize, char_len: usize) -> Self {
        Self { byte_len, char_len }
    }

    pub fn new_with_str(s: &str) -> Self {
        let char_len = s.chars().count();
        Self {
            byte_len: s.len(),
            char_len,
        }
    }
}

impl Default for Len {
    fn default() -> Self {
        Self {
            byte_len: 0,
            char_len: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    _pos: Pos,
    _len: Len,
}

impl Default for Range {
    fn default() -> Self {
        Self {
            _pos: Default::default(),
            _len: Default::default(),
        }
    }
}

impl Range {
    pub fn new(pos: Pos, len: Len) -> Self {
        Self {
            _pos: pos,
            _len: len,
        }
    }

    pub fn len(&self) -> Len {
        self._len
    }

    pub fn pos(&self) -> Pos {
        self._pos
    }

    #[inline]
    pub fn byte_start(&self) -> usize {
        self._pos.byte_pos
    }
    #[inline]
    pub fn byte_len(&self) -> usize {
        self._len.byte_len
    }
    #[inline]
    pub fn byte_end(&self) -> usize {
        self._pos.byte_pos + self._len.byte_len
    }

    #[inline]
    pub fn char_start(&self) -> usize {
        self._pos.char_pos
    }
    #[inline]
    pub fn char_len(&self) -> usize {
        self._len.char_len
    }
    #[inline]
    pub fn char_end(&self) -> usize {
        self._pos.char_pos + self._len.char_len
    }

    #[inline]
    pub fn line(&self) -> usize {
        self._pos.line
    }
    #[inline]
    pub fn column(&self) -> usize {
        self._pos.column
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenType {
    Ident,
    Int,
    Float,
    String,
    LPar,      // '('
    RPar,      // ')'
    LBrack,    // '['
    RBrack,    // ']'
    LBrace,    // '{'
    RBrace,    // '}'
    Dot,       // '.'
    Comma,     // ','
    Colon,     // ':'
    DbColon,   // '::'
    SemiColon, // ';'
    Add,       // '+'
    Minus,     // '-'
    Star,      // '*'
    Div,       // '/'
    IDiv,      // '//'
    Mod,       // '%'
    Pow,       // '**'
    Shl,       // '<<'
    Shr,       // '>>'
    BitAnd,    // '&'
    BitOr,     // '|'
    BitXor,    // '^'
    BitNot,    // '~'
    Cmp,       // '<=>'
    Lt,        // '<'
    Gt,        // '>'
    Le,        // '<='
    Ge,        // '>='
    Eq,        // '=='
    Ne,        // '!='
    Not,       // '!'
    And,       // '&&'
    Or,        // '||'
    Assign,    // '='
    Arrow,     // '=>'
    If,        // 'if'
    Else,      // 'else'
    While,     // 'while'
    For,       // 'For'
    Return,    // 'return'
    Function,  // "function"
    Type,      // "type"
    Public,    // "public"
    Eof,
    Comment,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Token<'s> {
    _type: TokenType,
    _range: Range,
    _source: &'s str,
}

impl<'s> Token<'s> {
    pub(crate) fn new(type_: TokenType, range: Range, source: &'s str) -> Self {
        Self {
            _type: type_,
            _range: range,
            _source: source,
        }
    }

    pub fn set_type(&mut self, type_: TokenType) {
        self._type = type_;
    }

    pub fn token_type(&self) -> TokenType {
        self._type
    }

    pub fn source(&self) -> &'s str {
        let beg = self._range.byte_start();
        let end = self._range.byte_start() + self._range.byte_len();
        &self._source[beg..end]
    }

    pub fn pos(&self) -> Pos {
        self._range.pos()
    }

    pub fn len(&self) -> Len {
        self._range.len()
    }

    pub fn byte_start(&self) -> usize {
        self._range.byte_start()
    }
    pub fn byte_len(&self) -> usize {
        self._range.byte_len()
    }

    pub fn char_start(&self) -> usize {
        self._range.char_start()
    }
    pub fn char_len(&self) -> usize {
        self._range.char_len()
    }

    pub fn line(&self) -> usize {
        self._range.line()
    }
    pub fn column(&self) -> usize {
        self._range.column()
    }

    pub fn as_int(&self) -> Option<i64> {
        self.source().parse::<i64>().ok()
    }

    pub fn as_float(&self) -> Option<f64> {
        self.source().parse::<f64>().ok()
    }

    pub fn as_string<'a>(&self, buf: &'a mut [u8]) -> Option<&'a str> {
        if self._type == TokenType::String {
            let mut source = self.source();
            source = source.strip_prefix('"').unwrap_or(source);
            source = source.strip_suffix('"').unwrap_or(source);

            let buf_len = buf.len();

            let mut encode_buf = [0; 8];
            let mut remnant = &mut buf[..];
            let mut chars = source.chars();
            while let Some(c) = chars.next() {
                let nc = if c == '\\' {
                    if let Some(c) = chars.next() {
                        match c {
                            'n' => '\n',
                            'r' => '\r',
                            't' => '\t',
                            _ => c,
                        }
                    } else {
                        return None;
                    }
                } else {
                    c
                };
                let cs = nc.encode_utf8(&mut encode_buf);
                if cs.len() > remnant.len() {
                    return None;
                }
                remnant[..cs.len()].copy_from_slice(cs.as_bytes());
                remnant = &mut remnant[cs.len()..];
            }
            let len = buf_len - remnant.len();
            let s = unsafe { core::str::from_utf8_unchecked(&buf[..len]) };
            Some(s)
        } else {
            None
        }
    }
}

impl<'s> core::fmt::Debug for Token<'s> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            "Token {{ type: {:?}, byte: {}:{}, char: {}:{}, pos: {}:{}, source: {:?} }}",
            self._type,
            self.byte_start(),
            self.byte_len(),
            self.char_start(),
            self.char_len(),
            self.line(),
            self.column(),
            self.source()
        ))
    }
}
