use core::fmt::Debug;
use core::fmt::Formatter;
use core::fmt::{Arguments as FmtArguments, Result as FmtResult};

use crate::token::Pos;

use crate::string::RString;
use crate::type_::*;
use crate::value::*;

#[derive(Clone, Debug)]
pub struct ParseError {
    _pos: Pos,
    _msg: Ref<RString>,
}

impl ParseError {
    pub fn pos(&self) -> Pos {
        self._pos
    }

    pub fn msg(&self) -> Ref<RString> {
        self._msg.clone()
    }
}

#[derive(Clone)]
pub enum Error {
    Initialize,
    OutOfMemory,
    OutOfRange,
    Parse(ParseError),
    Runtime(RValue),
    Type {
        expect: Ref<RType>,
        give: Ref<RType>,
    },
}

impl Error {
    pub fn new_outofmemory() -> Self {
        Self::OutOfMemory
    }

    pub fn new_outofrange() -> Self {
        Self::OutOfRange
    }

    pub fn new_parse(pos: Pos, msg: Ref<RString>) -> Self {
        Self::Parse(ParseError {
            _pos: pos,
            _msg: msg,
        })
    }

    pub fn new_runtime(err: RValue) -> Self {
        Self::Runtime(err)
    }

    pub fn new_type(expect: Ref<RType>, give: Ref<RType>) -> Self {
        Self::Type { expect, give }
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Initialize => f.write_str("initialize error"),
            Self::OutOfMemory => f.write_str("OutOfMemory"),
            Self::OutOfRange => f.write_str("OutOfRang"),
            Self::Parse(pe) => f.write_fmt(format_args!(
                "{}, at {}:{}",
                pe._msg.as_str(),
                pe._pos.line,
                pe._pos.column
            )),
            Self::Runtime(re) => f.write_fmt(format_args!("runtime error: {:?}", re)),
            Self::Type { expect, give } => f.write_fmt(format_args!(
                "type error, expect \"{:?}\", but give \"{:?}\"",
                expect.name(),
                give.name()
            )),
        }
    }
}

pub fn new_parse_error_str(pos: Pos, msg: &str) -> Error {
    match RString::new(msg) {
        Ok(v) => Error::new_parse(pos, v),
        Err(e) => e,
    }
}
pub fn new_parse_error_str_fmt(pos: Pos, args: FmtArguments) -> Error {
    match RString::format(args) {
        Ok(v) => Error::new_parse(pos, v),
        Err(e) => e,
    }
}

pub fn new_runtime_error_str(msg: &str) -> Error {
    match RString::new(msg) {
        Ok(v) => Error::new_runtime(v.cast_value()),
        Err(e) => e,
    }
}
pub fn new_runtime_error_str_fmt(args: FmtArguments) -> Error {
    match RString::format(args) {
        Ok(v) => Error::new_runtime(v.cast_value()),
        Err(e) => e,
    }
}

impl From<ParseError> for Error {
    fn from(e: ParseError) -> Self {
        Error::Parse(e)
    }
}

#[macro_export]
macro_rules! runtime_error_fmt {
    ($args:expr) => {
        new_runtime_error_str($args)
    };
    ($($args:tt)+) => {
        new_runtime_error_str_fmt(format_args!($($args)+))
    };
}

#[macro_export]
macro_rules! parse_error_fmt{
    ($pos: expr, $args:expr) => {
        new_parse_error_str($pos, $args)
    };
    ($pos: expr, $($args:tt)+) => {
        new_parse_error_str_fmt($pos, format_args!($($args)+))
    };
}
