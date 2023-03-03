use crate::token::*;

use crate::error::*;
use crate::parse_error_fmt;

#[allow(dead_code)]
mod util {
    pub fn to_char(s: &str) -> Result<char, &'static str> {
        s.chars().take(1).next().ok_or("invalid character")
    }

    pub fn convert_escape(s: &str) -> Result<char, &'static str> {
        match s {
            "\\n" => Ok('\n'),
            "\\r" => Ok('\r'),
            "\\t" => Ok('\t'),
            "\\\\" => Ok('\\'),
            "\\\"" => Ok('\"'),
            "\\\'" => Ok('\''),
            _ => Err("invalid escape character"),
        }
    }

    pub fn is_non_zore_digit(c: char) -> bool {
        c > '0' && c <= '9'
    }

    pub fn is_digit(c: char) -> bool {
        c >= '0' && c <= '9'
    }

    pub fn is_hex_digit(c: char) -> bool {
        (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F')
    }

    pub fn is_newline(s: &str) -> bool {
        s == "\n" || s == "\r" || s == "\r\n"
    }

    pub fn is_newline_char(c: char) -> bool {
        c == '\n' || c == '\r'
    }

    pub fn is_blank_char(c: char) -> bool {
        c == ' ' || c == '\t' || c == '\n' || c == '\r'
    }

    pub fn is_name_char(c: char) -> bool {
        !c.is_ascii_control()
            && !is_blank_char(c)
            && !"~!@#$%^&+-*/\\|=;:'\",.(){}[]<>?".contains(c)
    }

    pub fn is_name_char_non_num(c: char) -> bool {
        !is_digit(c) && is_name_char(c)
    }

    pub fn starts_match<F: Fn(char) -> bool>(input: &str, f: F) -> (Option<&str>, &str) {
        let byte_len = input
            .chars()
            .take_while(|c| f(*c))
            .map(|c| c.len_utf8())
            .fold(0, |a, n| a + n);

        if byte_len != 0 {
            (Some(&input[0..byte_len]), &input[byte_len..])
        } else {
            (None, input)
        }
    }
}

pub struct Lexical<'s> {
    _pos: Pos,
    _source: &'s str,
    _remnant: &'s str,
}

impl<'s> Lexical<'s> {
    pub fn new(source: &'s str) -> Self {
        Lexical {
            _pos: Pos::default(),
            _source: source,
            _remnant: source,
        }
    }

    pub fn current_pos(&self) -> Pos {
        self._pos
    }

    pub fn remnant(&self) -> &'s str {
        self._remnant
    }

    pub fn end(&self) -> bool {
        self.current_pos().byte_pos >= self._source.len()
    }

    fn match_op(&mut self) -> Result<Option<Token<'s>>, Error> {
        use TokenType as TT;

        let source = self._source.clone();
        let remnant = self.remnant();

        let tk = if remnant.starts_with("<=>") {
            Some((TT::Cmp, "<=>"))
        } else if remnant.starts_with("**") {
            Some((TT::Pow, "**"))
        } else if remnant.starts_with("//") {
            Some((TT::IDiv, "//"))
        } else if remnant.starts_with("<<") {
            Some((TT::Shl, "<<"))
        } else if remnant.starts_with(">>") {
            Some((TT::Shr, ">>"))
        } else if remnant.starts_with("<=") {
            Some((TT::Le, "<="))
        } else if remnant.starts_with(">=") {
            Some((TT::Ge, ">="))
        } else if remnant.starts_with("==") {
            Some((TT::Eq, "=="))
        } else if remnant.starts_with("!=") {
            Some((TT::Ne, "!="))
        } else if remnant.starts_with("&&") {
            Some((TT::And, "&&"))
        } else if remnant.starts_with("||") {
            Some((TT::Or, "||"))
        } else if remnant.starts_with("=>") {
            Some((TT::Arrow, "=>"))
        } else if remnant.starts_with("::") {
            Some((TT::DbColon, "::"))
        } else if remnant.starts_with("(") {
            Some((TT::LPar, "("))
        } else if remnant.starts_with(")") {
            Some((TT::RPar, ")"))
        } else if remnant.starts_with("[") {
            Some((TT::LBrack, "["))
        } else if remnant.starts_with("]") {
            Some((TT::RBrack, "]"))
        } else if remnant.starts_with("{") {
            Some((TT::LBrace, "{"))
        } else if remnant.starts_with("}") {
            Some((TT::RBrace, "}"))
        } else if remnant.starts_with(".") {
            Some((TT::Dot, "."))
        } else if remnant.starts_with(",") {
            Some((TT::Comma, ","))
        } else if remnant.starts_with(":") {
            Some((TT::Colon, ":"))
        } else if remnant.starts_with(";") {
            Some((TT::SemiColon, ";"))
        } else if remnant.starts_with("+") {
            Some((TT::Add, "+"))
        } else if remnant.starts_with("-") {
            Some((TT::Minus, "-"))
        } else if remnant.starts_with("*") {
            Some((TT::Star, "*"))
        } else if remnant.starts_with("/") {
            Some((TT::Div, "/"))
        } else if remnant.starts_with("%") {
            Some((TT::Mod, "%"))
        } else if remnant.starts_with("&") {
            Some((TT::BitAnd, "&"))
        } else if remnant.starts_with("|") {
            Some((TT::BitOr, "|"))
        } else if remnant.starts_with("^") {
            Some((TT::BitXor, "^"))
        } else if remnant.starts_with("~") {
            Some((TT::BitNot, "~"))
        } else if remnant.starts_with("<") {
            Some((TT::Lt, "<"))
        } else if remnant.starts_with(">") {
            Some((TT::Gt, ">"))
        } else if remnant.starts_with("!") {
            Some((TT::Not, "!"))
        } else if remnant.starts_with("=") {
            Some((TT::Assign, "="))
        } else {
            None
        };

        if let Some((tt, src)) = tk {
            let range = Range::new(self._pos, Len::new_with_str(src));
            Ok(Some(Token::new(tt, range, source)))
        } else {
            Ok(None)
        }
    }

    fn match_name(&mut self) -> Result<Option<Token<'s>>, Error> {
        let mut iter = self.remnant().char_indices();

        let (_, first) = if let Some(r) = iter.next() {
            r
        } else {
            return Ok(None);
        };

        if !util::is_name_char_non_num(first) {
            return Ok(None);
        }

        let mut char_len = 1;
        let mut byte_len = 0;
        let mut last_char = first;

        while let Some((pos, char_)) = iter.next() {
            if !util::is_name_char(char_) {
                break;
            }
            char_len += 1;
            last_char = char_;
            byte_len = pos;
        }

        let len = Len::new(byte_len + last_char.len_utf8(), char_len);
        let range = Range::new(self._pos, len);
        let mut tk = Token::new(TokenType::Ident, range, self._source.clone());

        if tk.source() == "if" {
            tk.set_type(TokenType::If);
        } else if tk.source() == "else" {
            tk.set_type(TokenType::Else);
        } else if tk.source() == "while" {
            tk.set_type(TokenType::While);
        } else if tk.source() == "for" {
            tk.set_type(TokenType::For);
        } else if tk.source() == "return" {
            tk.set_type(TokenType::Return);
        } else if tk.source() == "function" {
            tk.set_type(TokenType::Function);
        } else if tk.source() == "type" {
            tk.set_type(TokenType::Type);
        } else if tk.source() == "public" {
            tk.set_type(TokenType::Public);
        }

        Ok(Some(tk))
    }

    fn match_int(&mut self) -> Result<Option<Token<'s>>, Error> {
        let mut remnant = self.remnant();

        let mut byte_len = 0;
        let mut char_len = 0;

        if remnant.starts_with("-") {
            byte_len += 1;
            char_len += 1;
            remnant = &remnant[1..];
        };

        if remnant.starts_with("0x") || remnant.starts_with("0X") {
            byte_len += 2;
            char_len += 2;
            remnant = &remnant[2..];

            let (hexs, other) = util::starts_match(remnant, util::is_hex_digit);

            if let Some(hexs) = hexs {
                byte_len += hexs.len();
                char_len += hexs.chars().count();

                if let Some(c) = other.chars().next() {
                    if util::is_name_char(c) {
                        let mut pos = self._pos;
                        pos.byte_pos += byte_len;
                        pos.char_pos += char_len;
                        pos.column += char_len;
                        return Err(parse_error_fmt!(
                            pos,
                            "invalid numeric literal character: {:?}",
                            c
                        ));
                    }
                }
            } else {
                let mut pos = self._pos;
                pos.byte_pos += byte_len;
                pos.char_pos += char_len;
                pos.column += char_len;
                return Err(parse_error_fmt!(
                    pos,
                    "missing digits after the integer base prefix",
                ));
            }
        } else {
            let (digits, other) = util::starts_match(remnant, util::is_digit);

            if let Some(digits) = digits {
                if let Some(c) = other.chars().next() {
                    if util::is_name_char(c) {
                        let mut pos = self._pos;
                        pos.byte_pos += byte_len;
                        pos.char_pos += char_len;
                        pos.column += char_len;
                        return Err(parse_error_fmt!(
                            pos,
                            "invalid numeric literal character: {:?}",
                            c
                        ));
                    }
                }

                byte_len += digits.len();
                char_len += digits.chars().count();
            } else {
                return Ok(None);
            }
        }

        let range = Range::new(self._pos, Len::new(byte_len, char_len));
        Ok(Some(Token::new(
            TokenType::Int,
            range,
            self._source.clone(),
        )))
    }

    fn match_float(&mut self) -> Result<Option<Token<'s>>, Error> {
        let mut remnant = self.remnant();

        let mut byte_len = 0;
        let mut char_len = 0;

        if remnant.starts_with("-") || remnant.starts_with("+") {
            byte_len += 1;
            char_len += 1;
            remnant = &remnant[1..];
        };

        let (digits, other) = util::starts_match(remnant, util::is_digit);

        if let Some(digits) = digits {
            byte_len += digits.len();
            char_len += digits.chars().count();
        } else {
            return Ok(None);
        }

        if other.starts_with(".") {
            byte_len += 1;
            char_len += 1;
            remnant = &other[1..];

            let (digits, other) = util::starts_match(remnant, util::is_digit);
            if let Some(digits) = digits {
                byte_len += digits.len();
                char_len += digits.chars().count();
            } else if other.len() == 0 {
                let mut pos = self._pos;
                pos.byte_pos += byte_len;
                pos.char_pos += char_len;
                pos.column += char_len;
                return Err(parse_error_fmt!(
                    pos,
                    "incomplete numeric literal character"
                ));
            } else {
                let mut pos = self._pos;
                pos.byte_pos += byte_len;
                pos.char_pos += char_len;
                pos.column += char_len;
                return Err(parse_error_fmt!(
                    pos,
                    "invalid numeric literal character: {:?}",
                    other.chars().next().unwrap()
                ));
            }
        } else {
            return Ok(None);
        }

        let range = Range::new(self._pos, Len::new(byte_len, char_len));

        Ok(Some(Token::new(
            TokenType::Float,
            range,
            self._source.clone(),
        )))
    }

    fn match_string(&mut self) -> Result<Option<Token<'s>>, Error> {
        let mut remnant = self.remnant();

        let mut byte_len = 0;
        let mut char_len = 0;

        if remnant.starts_with("\"") {
            byte_len += 1;
            char_len += 1;
            remnant = &remnant[1..];
        } else {
            return Ok(None);
        }

        let mut closed = false;
        let mut chars = remnant.chars().peekable();
        while let Some(c) = chars.next() {
            byte_len += c.len_utf8();
            char_len += 1;
            if c == '\\' {
                if let Some(c) = chars.next() {
                    byte_len += c.len_utf8();
                    char_len += 1;
                } else {
                    let mut pos = self._pos;
                    pos.byte_pos += byte_len;
                    pos.char_pos += char_len;
                    pos.column += char_len;
                    return Err(parse_error_fmt!(pos, "unclosed string literals"));
                }
            } else if c == '"' {
                closed = true;
                break;
            }
        }

        if !closed {
            let mut pos = self._pos;
            pos.byte_pos += byte_len;
            pos.char_pos += char_len;
            pos.column += char_len;
            return Err(parse_error_fmt!(pos, "unclosed string literals"));
        }

        let range = Range::new(self._pos, Len::new(byte_len, char_len));

        Ok(Some(Token::new(
            TokenType::String,
            range,
            self._source.clone(),
        )))
    }

    fn match_comment(&mut self) -> Result<Option<Token<'s>>, Error> {
        let remnant = self.remnant();
        if !remnant.starts_with("#") {
            return Ok(None);
        }

        let mut char_count = 0;
        let mut last_pos: usize = 0;
        let mut last_char: char = '#';

        let mut chars = remnant.char_indices();
        while let Some((pos, char_)) = chars.next() {
            if util::is_newline_char(char_) {
                break;
            }
            char_count += 1;
            last_char = char_;
            last_pos = pos;
        }

        let byte_len = last_pos + last_char.len_utf8();
        let char_len = char_count;

        let range = Range::new(self._pos, Len::new(byte_len, char_len));

        Ok(Some(Token::new(
            TokenType::Comment,
            range,
            self._source.clone(),
        )))
    }

    fn _skip_blank(&mut self) {
        let mut old_pos = self._pos;
        let mut bs = 0;
        let mut cs = 0;
        let mut l = old_pos.line;
        let mut c = old_pos.column;

        let mut chars = self.remnant().chars().peekable();
        loop {
            match chars.next() {
                Some('\r') if matches!(chars.peek(), Some('\n')) => {
                    bs += 2;
                    cs += 2;
                    l += 1;
                    c = 1;
                    chars.next();
                }
                Some('\r' | '\n') => {
                    bs += 1;
                    cs += 1;
                    l += 1;
                    c = 1;
                }
                Some(ch) if util::is_blank_char(ch) => {
                    bs += ch.len_utf8();
                    cs += 1;
                    c += 1;
                }
                _ => break,
            }
        }

        old_pos.byte_pos += bs;
        old_pos.char_pos += cs;
        old_pos.line = l;
        old_pos.column = c;
        self._pos = old_pos;
        self._remnant = &self._source[old_pos.byte_pos..];
    }

    fn _next_token(&mut self) -> Result<Option<Token<'s>>, Error> {
        self._skip_blank();

        if self.remnant().is_empty() {
            return Ok(None);
        }

        let res = if false {
            Ok(None)
        } else if let Some(tk) = self.match_float()? {
            Ok(Some(tk))
        } else if let Some(tk) = self.match_int()? {
            Ok(Some(tk))
        } else if let Some(tk) = self.match_name()? {
            Ok(Some(tk))
        } else if let Some(tk) = self.match_op()? {
            Ok(Some(tk))
        } else if let Some(tk) = self.match_string()? {
            Ok(Some(tk))
        } else if let Some(tk) = self.match_comment()? {
            Ok(Some(tk))
        } else {
            let len = self
                .remnant()
                .chars()
                .take(16)
                .map(|c| c.len_utf8())
                .fold(0, |a, n| a + n);
            Err(parse_error_fmt!(
                self._pos,
                "exceeds expected string: {}...",
                &self.remnant()[..len],
            ))
        };

        if let Ok(Some(tk)) = &res {
            self._forward(tk.len());
        }

        res
    }

    fn _forward(&mut self, len: Len) {
        self._pos.byte_pos += len.byte_len;
        self._pos.char_pos += len.char_len;
        self._pos.column += len.char_len;
        self._remnant = &self._source[self._pos.byte_pos..];
    }

    pub fn next_token(&mut self) -> Result<Option<Token<'s>>, Error> {
        let res = self._next_token()?;
        self._skip_blank();
        Ok(res)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::alloc;
    use crate::alloc::Allocator;
    use crate::runtime::Loader;
    use crate::runtime::{finalize, initialize};
    use crate::token::TokenType as TT;

    fn allocator() -> &'static dyn Allocator {
        alloc::default_allocator()
    }

    fn loader() -> &'static mut dyn Loader {
        struct L;
        impl Loader for L {
            fn normalize_name(
                &mut self,
                _requester: crate::Ref<crate::RModule>,
                _name: crate::Ref<crate::RString>,
            ) -> Result<crate::Ref<crate::RString>, Error> {
                todo!()
            }
            fn load(
                &mut self,
                _normalized_name: crate::Ref<crate::RString>,
            ) -> Result<crate::Ref<crate::RFunction>, Error> {
                todo!()
            }
        }
        use core::ptr::addr_of_mut;
        let mut l = L;
        unsafe { &mut *addr_of_mut!(l) }
    }

    fn get_type(res: &Result<Option<Token>, Error>) -> Option<TokenType> {
        if let Ok(Some(t)) = res {
            Some(t.token_type())
        } else {
            None
        }
    }

    fn get_source<'s>(res: &Result<Option<Token<'s>>, Error>) -> Option<&'s str> {
        if let Ok(Some(t)) = res {
            Some(t.source())
        } else {
            None
        }
    }

    #[test]
    fn test_op() {
        let source = ">> > << < <= => == != * ** & &&";

        initialize(allocator(), loader()).unwrap();

        let mut lexical = Lexical::new(source);

        assert_eq!(get_type(&lexical.next_token()), Some(TT::Shr));
        assert_eq!(get_type(&lexical.next_token()), Some(TT::Gt));
        assert_eq!(get_type(&lexical.next_token()), Some(TT::Shl));
        assert_eq!(get_type(&lexical.next_token()), Some(TT::Lt));
        assert_eq!(get_type(&lexical.next_token()), Some(TT::Le));
        assert_eq!(get_type(&lexical.next_token()), Some(TT::Arrow));
        assert_eq!(get_type(&lexical.next_token()), Some(TT::Eq));
        assert_eq!(get_type(&lexical.next_token()), Some(TT::Ne));
        assert_eq!(get_type(&lexical.next_token()), Some(TT::Star));
        assert_eq!(get_type(&lexical.next_token()), Some(TT::Pow));
        assert_eq!(get_type(&lexical.next_token()), Some(TT::BitAnd));
        assert_eq!(get_type(&lexical.next_token()), Some(TT::And));
        assert_eq!(get_type(&lexical.next_token()), None);

        finalize();
    }

    #[test]
    fn test_int() {
        let source = "1234 -1234 01234 0 1 -1 -0 0x1234 0X1234 -0x1234 -0X1234 0x89ab 0xffff";
        initialize(allocator(), loader()).unwrap();

        let mut lexical = Lexical::new(source);

        for s in source.split_ascii_whitespace() {
            let t = lexical.next_token();
            assert_eq!(get_type(&t), Some(TT::Int));
            assert_eq!(get_source(&t), Some(s));
        }
        finalize();
    }

    #[test]
    fn test_invalid_int() {
        let source = "0x";
        initialize(allocator(), loader()).unwrap();

        let mut lexical = Lexical::new(source);
        assert!(lexical.next_token().is_err());
        finalize();
    }

    #[test]
    fn test_float() {
        let source = "0.0 -0.0 01.0 -01.0 1.1 -1.1 -0.2313";
        initialize(allocator(), loader()).unwrap();

        let mut lexical = Lexical::new(source);

        for s in source.split_ascii_whitespace() {
            let t = lexical.next_token();
            assert_eq!(get_type(&t), Some(TT::Float));
            assert_eq!(get_source(&t), Some(s));
        }

        finalize();
    }

    #[test]
    fn test_invalid_float() {
        initialize(allocator(), loader()).unwrap();

        {
            let source = "0.";
            let mut lexical = Lexical::new(source);

            assert!(lexical.next_token().is_err());
        }
        {
            let source = ".0";
            let mut lexical = Lexical::new(source);

            assert_eq!(get_type(&lexical.next_token()), Some(TT::Dot));
        }
        {
            let source = "-1.aa";
            let mut lexical = Lexical::new(source);

            assert!(lexical.next_token().is_err());
        }
        finalize();
    }

    #[test]
    fn test_string() {
        let source = r#""asd" "\"asdf" "asdf\"" "asdf\n" "\n\r\t\\\"\a""#;
        initialize(allocator(), loader()).unwrap();
        let mut lexical = Lexical::new(source);

        for s in source.split_ascii_whitespace() {
            let t = lexical.next_token();
            assert_eq!(get_type(&t), Some(TT::String));
            assert_eq!(get_source(&t), Some(s));
        }

        finalize();
    }
}
