use crate::collections::Array;

use crate::runtime::*;

use crate::op::*;

use crate::error::*;
use crate::{parse_error_fmt, runtime_error_fmt};

use crate::ast::{Ast, RAst};
use crate::lexical::*;
use crate::token::TokenType as TT;
use crate::token::{Pos, Token};

use crate::number::*;
use crate::string::*;
use crate::value::*;

fn _tk_to_arith_op(tk: Token) -> Option<ArithOp> {
    match tk.token_type() {
        TT::Add => Some(ArithOp::Add),
        TT::Minus => Some(ArithOp::Sub),
        TT::Star => Some(ArithOp::Mul),
        TT::Div => Some(ArithOp::Div),
        TT::IDiv => Some(ArithOp::IDiv),
        TT::Mod => Some(ArithOp::Mod),
        TT::Pow => Some(ArithOp::Pow),
        TT::And => Some(ArithOp::And),
        TT::Or => Some(ArithOp::Or),
        TT::BitAnd => Some(ArithOp::BitAnd),
        TT::BitOr => Some(ArithOp::BitOr),
        TT::BitXor => Some(ArithOp::BitXor),
        TT::Shl => Some(ArithOp::Shl),
        TT::Shr => Some(ArithOp::Shr),
        _ => None,
    }
}

fn _tk_to_cmp_op(tk: Token) -> Option<CmpOp> {
    match tk.token_type() {
        TT::Cmp => Some(CmpOp::Cmp),
        TT::Eq => Some(CmpOp::Eq),
        TT::Ne => Some(CmpOp::Ne),
        TT::Lt => Some(CmpOp::Lt),
        TT::Le => Some(CmpOp::Le),
        TT::Gt => Some(CmpOp::Gt),
        TT::Ge => Some(CmpOp::Ge),
        _ => None,
    }
}

fn _tk_to_overload_arith_op(tk: Token) -> Option<OverloadOp> {
    match tk.token_type() {
        TT::Add => Some(OverloadOp::Add),
        TT::Minus => Some(OverloadOp::Sub),
        TT::Star => Some(OverloadOp::Mul),
        TT::Div => Some(OverloadOp::Div),
        TT::IDiv => Some(OverloadOp::IDiv),
        TT::Mod => Some(OverloadOp::Mod),
        TT::Pow => Some(OverloadOp::Pow),
        TT::And => Some(OverloadOp::And),
        TT::Or => Some(OverloadOp::Or),
        TT::BitAnd => Some(OverloadOp::BitAnd),
        TT::BitOr => Some(OverloadOp::BitOr),
        TT::BitXor => Some(OverloadOp::BitXor),
        TT::Shl => Some(OverloadOp::Shl),
        TT::Shr => Some(OverloadOp::Shr),
        _ => None,
    }
}

pub struct Parser<'s> {
    _lexical: Lexical<'s>,
    _tokens: Array<Token<'s>>,
    _cur_idx: usize,
}

impl<'s> Parser<'s> {
    pub fn new(lexical: Lexical<'s>) -> Self {
        let allocator = allocator();
        Self {
            _lexical: lexical,
            _tokens: Array::new(allocator),
            _cur_idx: 0,
        }
    }

    fn current_pos(&self) -> Pos {
        if let Some(pos) = self._tokens.get(self._cur_idx).map(|tk| tk.pos()) {
            pos
        } else {
            self._lexical.current_pos()
        }
    }

    fn end(&self) -> bool {
        self._cur_idx >= self._tokens.len() && self._lexical.end()
    }

    fn _peek_to_n(&mut self, n: usize) -> Result<(), Error> {
        let tks_len = self._tokens.len();
        let target_len = self._cur_idx + n;
        if target_len < tks_len {
            return Ok(());
        }

        for _ in tks_len..=target_len {
            if let Some(tk) = self._lexical.next_token()? {
                self._tokens.push(tk).map_err(|_| Error::OutOfMemory)?;
            } else {
                break;
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn next_token(&mut self) -> Result<Option<Token<'s>>, Error> {
        self._peek_to_n(0)?;
        if self._cur_idx >= self._tokens.len() {
            return Ok(None);
        }
        self._cur_idx += 1;
        let tk = self._tokens.get(self._cur_idx - 1).cloned();
        Ok(tk)
    }

    fn peek_token(&mut self, n: usize) -> Result<Option<Token<'s>>, Error> {
        self._peek_to_n(n)?;
        if self._cur_idx + n >= self._tokens.len() {
            return Ok(None);
        }
        let tk = self._tokens.get(self._cur_idx + n).cloned().unwrap();
        Ok(Some(tk))
    }

    fn expect(&mut self, type_: TT) -> Result<Token<'s>, Error> {
        if let Some(tk) = self.peek_token(0)? {
            if tk.token_type() == type_ {
                self._cur_idx += 1;
                Ok(tk)
            } else {
                Err(parse_error_fmt!(
                    tk.pos(),
                    "expect {:?}, but {:?} occurs",
                    type_,
                    tk.token_type(),
                ))
            }
        } else {
            let pos = self.current_pos();
            Err(parse_error_fmt!(
                pos,
                "expect {:?}, but at the end of the file",
                type_,
            ))
        }
    }

    fn expect_any(&mut self, types: &[TT]) -> Result<Token<'s>, Error> {
        debug_assert!(types.len() != 0, "the TokenType list cannot be empty.");

        if let Some(tk) = self.peek_token(0)? {
            if types.contains(&tk.token_type()) {
                self._cur_idx += 1;
                Ok(tk)
            } else {
                Err(parse_error_fmt!(
                    tk.pos(),
                    "expect {:?}, but {:?} occurs",
                    types,
                    tk.token_type(),
                ))
            }
        } else {
            let pos = self.current_pos();
            Err(parse_error_fmt!(
                pos,
                "expect {:?}, but at the end of the file",
                types[0],
            ))
        }
    }

    fn match_all(&mut self, token_types: &[TT]) -> bool {
        for i in 0..token_types.len() {
            match self.peek_token(i) {
                Ok(Some(tk)) if tk.token_type() == token_types[i] => (),
                _ => return false,
            }
        }
        true
    }

    fn match_any(&mut self, token_types: &[TT]) -> bool {
        match self.peek_token(0) {
            Ok(Some(tk)) if token_types.contains(&tk.token_type()) => true,
            _ => return false,
        }
    }

    fn match_(&mut self, token_type: TT) -> bool {
        match self.peek_token(0) {
            Ok(Some(tk)) if tk.token_type() == token_type => true,
            _ => return false,
        }
    }

    fn skip(&mut self, n: usize) -> Result<(), Error> {
        if n == 0 {
            return Ok(());
        }

        self._peek_to_n(n - 1)?;
        if self._cur_idx + n > self._tokens.len() {
            let pos = self.current_pos();
            return Err(parse_error_fmt!(pos, "unexpected end"));
        }
        self._cur_idx += n;
        Ok(())
    }

    pub fn parse(&mut self, allow_last_expr: bool) -> Result<Ref<RAst>, Error> {
        program(self, allow_last_expr)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Desc {
    VarExpr,
    Expr,
    StatExpr,
    Stat,
}

impl Desc {
    fn is_var_expr(&self) -> bool {
        *self == Desc::VarExpr
    }
    fn is_expr(&self) -> bool {
        *self == Desc::VarExpr || *self == Desc::Expr || *self == Self::StatExpr
    }
    fn is_stat(&self) -> bool {
        *self == Desc::StatExpr || *self == Self::Stat
    }
}

type PResult = Result<(Desc, Ref<RAst>), Error>;
type ExPResult<T> = Result<(Desc, Ref<RAst>, T), Error>;

fn program(parser: &mut Parser, allow_last_expr: bool) -> Result<Ref<RAst>, Error> {
    let mut stats = Array::new(allocator());
    let mut expr = None;

    comment(parser)?;

    while !parser.end() {
        let (desc, ast) = if parser.match_(TT::Public) {
            parser.next_token()?;
            let (desc, expr, name) = if parser.match_all(&[TT::Ident, TT::Assign]) {
                ident_assign(parser)?
            } else if parser.match_(TT::Function) {
                function_def(parser)?
            } else if parser.match_(TT::Type) {
                type_def(parser)?
            } else {
                return Err(parse_error_fmt!(
                    parser.current_pos(),
                    "public only supports assignment statements, function definitions and type definitions"
                ));
            };

            #[allow(unused_must_use)]
            if desc.is_stat() {
                parser.expect(TT::SemiColon);
            } else {
                parser.expect(TT::SemiColon)?;
            };

            let ast = RAst::new(Ast::ProgramPublic { name, expr })?;
            (Desc::Stat, ast)
        } else {
            _stat_or_expr(parser)?
        };

        let pos = parser.current_pos();

        comment(parser)?;

        if allow_last_expr && desc.is_expr() && parser.end() {
            expr = Some(ast);
            break;
        } else if desc.is_stat() {
            stats.push(ast).map_err(|_| Error::OutOfMemory)?;
        } else {
            return Err(parse_error_fmt!(pos, "missing \";\""));
        }
    }

    let ast = RAst::new(Ast::Program { stats, expr })?;

    Ok(ast)
}

fn ident_assign(parser: &mut Parser) -> ExPResult<Ref<RString>> {
    let name_tk = parser.expect(TT::Ident)?;
    parser.expect(TT::Assign)?;
    let (expr_desc, expr_ast) = expr(parser)?;

    let name = RString::new(name_tk.source())?;
    let ident_ast = RAst::new(Ast::Ident { name: name.clone() })?;

    let ast = RAst::new(Ast::Assign {
        target: ident_ast,
        expr: expr_ast,
    })?;

    let desc = if expr_desc.is_var_expr() {
        Desc::Expr
    } else {
        expr_desc
    };

    Ok((desc, ast, name.clone()))
}

fn comment(parser: &mut Parser) -> Result<(), Error> {
    while let Ok(_) = parser.expect(TT::Comment) {}
    Ok(())
}

fn stat(parser: &mut Parser, allow_return: bool) -> PResult {
    let (desc, expr) = if parser.match_(TT::SemiColon) {
        parser.next_token()?;
        let ast = RAst::new(Ast::Stat { expr: None })?;
        Ok((Desc::Stat, ast))
    } else if parser.match_(TT::If) {
        if_(parser, false)
    } else if parser.match_(TT::While) {
        while_(parser, false)
    } else if parser.match_(TT::For) {
        for_(parser, false)
    } else if parser.match_(TT::Return) {
        if allow_return {
            return_(parser)
        } else {
            return Err(parse_error_fmt!(
                parser.current_pos(),
                "\"return\" cannot be used at the top level"
            ));
        }
    } else {
        expr(parser)
    }?;

    #[allow(unused_must_use)]
    let stat_ast = if desc.is_stat() {
        parser.expect(TT::SemiColon);
        if desc.is_expr() {
            RAst::new(Ast::Stat { expr: Some(expr) })?
        } else {
            expr
        }
    } else {
        parser.expect(TT::SemiColon)?;
        RAst::new(Ast::Stat { expr: Some(expr) })?
    };

    Ok((Desc::Stat, stat_ast))
}

fn expr(parser: &mut Parser) -> PResult {
    if _match_lambda(parser) {
        lambda(parser)
    } else if parser.match_(TT::If) {
        if_(parser, true)
    } else if parser.match_(TT::While) {
        while_(parser, true)
    } else if parser.match_(TT::For) {
        for_(parser, true)
    } else if parser.match_(TT::Function) {
        function_def(parser).map(|(a, b, _)| (a, b))
    } else if parser.match_(TT::Type) {
        type_def(parser).map(|(a, b, _)| (a, b))
    } else {
        let (desc, ast) = binary_expr(parser, MAX_BINOP_LEVEL)?;
        if desc.is_var_expr() && parser.expect(TT::Assign).is_ok() {
            let (expr_desc, expr) = expr(parser)?;
            let ast = RAst::new(Ast::Assign { target: ast, expr })?;
            if expr_desc.is_var_expr() {
                Ok((Desc::Expr, ast))
            } else {
                Ok((expr_desc, ast))
            }
        } else {
            Ok((desc, ast))
        }
    }
}

/// 如果must_expr为true，则返回的 desc.is_expr() == true，
/// 否则，返回的 desc.is_expr() == true || desc.is_expr() == false,
/// 在must_expr为true的情况下，会尽可能的匹配语句。
fn _stat_or_expr(parser: &mut Parser) -> PResult {
    if parser.match_(TT::SemiColon) {
        parser.next_token()?;
        let ast = RAst::new(Ast::Stat { expr: None })?;
        return Ok((Desc::Stat, ast));
    } else if parser.match_(TT::Return) {
        return return_(parser);
    }

    let (desc, ast) = {
        if _match_lambda(parser) {
            lambda(parser)
        } else if parser.match_(TT::If) {
            if_(parser, false)
        } else if parser.match_(TT::While) {
            while_(parser, false)
        } else if parser.match_(TT::For) {
            for_(parser, false)
        } else if parser.match_(TT::Function) {
            function_def(parser).map(|(a, b, _)| (a, b))
        } else if parser.match_(TT::Type) {
            type_def(parser).map(|(a, b, _)| (a, b))
        } else {
            let (desc, ast) = binary_expr(parser, MAX_BINOP_LEVEL)?;
            if desc.is_var_expr() && parser.expect(TT::Assign).is_ok() {
                let (expr_desc, expr) = expr(parser)?;
                let ast = RAst::new(Ast::Assign { target: ast, expr })?;
                if expr_desc.is_var_expr() {
                    Ok((Desc::Expr, ast))
                } else {
                    Ok((expr_desc, ast))
                }
            } else {
                Ok((desc, ast))
            }
        }
    }?;

    let desc = if desc.is_expr() {
        if parser.match_(TT::SemiColon) {
            parser.next_token()?;
            Desc::Stat
        } else {
            desc
        }
    } else {
        desc
    };

    Ok((desc, ast))
}

fn return_(parser: &mut Parser) -> PResult {
    parser.expect(TT::Return)?;
    if parser.match_(TT::SemiColon) {
        parser.next_token()?;
        let ast = RAst::new(Ast::Return { expr: None })?;
        Ok((Desc::Expr, ast))
    } else {
        let (expr_desc, expr_ast) = expr(parser)?;

        #[allow(unused_must_use)]
        if expr_desc.is_stat() {
            parser.expect(TT::SemiColon);
        } else {
            parser.expect(TT::SemiColon)?;
        }

        let ast = RAst::new(Ast::Return {
            expr: Some(expr_ast),
        })?;
        Ok((Desc::Stat, ast))
    }
}

fn _match_lambda(parser: &mut Parser) -> bool {
    if parser.match_all(&[TT::Ident, TT::Arrow]) {
        return true;
    }

    let mut stack = 0;
    let mut i = 0;
    while let Ok(Some(tk)) = parser.peek_token(i) {
        if tk.token_type() == TT::LPar {
            stack += 1;
        } else if tk.token_type() == TT::RPar {
            stack -= 1;
        }

        if stack == 0 {
            if let Ok(Some(tk)) = parser.peek_token(i + 1) {
                if tk.token_type() == TT::Arrow {
                    return true;
                } else {
                    return false;
                }
            }
            break;
        }

        i += 1;
    }
    return false;
}

fn _paramets_list(parser: &mut Parser) -> Result<Array<Ref<RString>>, Error> {
    let mut paramets = Array::new(allocator());
    parser.expect(TT::LPar)?;
    if let Ok(name_tk) = parser.expect(TT::Ident) {
        let name = RString::new(name_tk.source())?;
        paramets.push(name).map_err(|_| Error::OutOfMemory)?;

        while parser.expect(TT::Comma).is_ok() {
            if let Ok(name_tk) = parser.expect(TT::Ident) {
                let name = RString::new(name_tk.source())?;
                paramets.push(name).map_err(|_| Error::OutOfMemory)?;
            }
        }
    }
    parser.expect(TT::RPar)?;
    Ok(paramets)
}

fn lambda(parser: &mut Parser) -> PResult {
    let paramets = if let Ok(name_tk) = parser.expect(TT::Ident) {
        let mut paramets = Array::new(allocator());
        let name = RString::new(name_tk.source())?;
        paramets.push(name).map_err(|_| Error::OutOfMemory)?;
        paramets
    } else {
        _paramets_list(parser)?
    };

    parser.expect(TT::Arrow)?;

    comment(parser)?;

    let (body_desc, body) = expr(parser)?;

    let desc = if body_desc.is_var_expr() {
        Desc::Expr
    } else {
        body_desc
    };

    let ast = RAst::new(Ast::Lambda { paramets, body })?;

    Ok((desc, ast))
}

fn function_def(parser: &mut Parser) -> ExPResult<Ref<RString>> {
    parser.expect(TT::Function)?;

    let name_tk = parser.expect(TT::Ident)?;
    let name = RString::new(name_tk.source())?;

    let paramets = _paramets_list(parser)?;

    comment(parser)?;

    let (_, body) = block_expr(parser)?;

    let ast = RAst::new(Ast::FunctionDef {
        name: name.clone(),
        paramets,
        body,
    })?;

    Ok((Desc::StatExpr, ast, name))
}

fn _expect_overload_operator_def(parser: &mut Parser) -> Result<Option<OverloadOp>, Error> {
    if false {
        Ok(None)
    } else if parser.match_all(&[TT::Function, TT::LBrack, TT::Ident, TT::RBrack]) {
        if let Ok(Some(tk)) = parser.peek_token(2) {
            if tk.source() == "new" {
                parser.skip(4)?;
                Ok(Some(OverloadOp::New))
            } else if tk.source() == "destory" {
                parser.skip(4)?;
                Ok(Some(OverloadOp::Destory))
            } else if tk.source() == "str" {
                parser.skip(4)?;
                Ok(Some(OverloadOp::Str))
            } else if tk.source() == "hash" {
                parser.skip(4)?;
                Ok(Some(OverloadOp::Hash))
            } else if tk.source() == "iter" {
                parser.skip(4)?;
                Ok(Some(OverloadOp::Iter))
            } else if tk.source() == "next" {
                parser.skip(4)?;
                Ok(Some(OverloadOp::Next))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    } else if parser.match_all(&[TT::Function, TT::LBrack, TT::RBrack, TT::Assign]) {
        parser.skip(4)?;
        Ok(Some(OverloadOp::SetItem))
    } else if parser.match_all(&[TT::Function, TT::LBrack, TT::RBrack]) {
        parser.skip(3)?;
        Ok(Some(OverloadOp::GetItem))
    } else if parser.match_all(&[TT::Function, TT::LPar, TT::RPar]) {
        parser.skip(3)?;
        Ok(Some(OverloadOp::Call))
    } else if parser.match_all(&[TT::Function, TT::Not]) {
        parser.skip(2)?;
        Ok(Some(OverloadOp::Not))
    } else if parser.match_all(&[TT::Function, TT::BitNot]) {
        parser.skip(2)?;
        Ok(Some(OverloadOp::BitNot))
    } else if parser.match_(TT::Function) {
        if let Ok(Some(tk)) = parser.peek_token(1) {
            if let Some(op) = _tk_to_overload_arith_op(tk) {
                parser.skip(2)?;
                Ok(Some(op))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

fn type_def(parser: &mut Parser) -> ExPResult<Ref<RString>> {
    parser.expect(TT::Type)?;

    let name_tk = parser.expect(TT::Ident)?;
    let name = RString::new(name_tk.source())?;

    let mut stats = Array::new(allocator());

    parser.expect(TT::LBrace)?;

    comment(parser)?;

    while !parser.match_(TT::RBrace) {
        let (_, ast) = if parser.match_(TT::Public) {
            parser.next_token()?;
            let (desc, expr, name) = if parser.match_all(&[TT::Ident, TT::Assign]) {
                ident_assign(parser)?
            } else if parser.match_(TT::Function) {
                function_def(parser)?
            // } else if parser.match_(TT::Type) {
            //     type_def(parser)?
            } else {
                return Err(parse_error_fmt!(
                    parser.current_pos(),
                    "public only supports assignment statements and function definitions"
                ));
            };

            #[allow(unused_must_use)]
            if desc.is_stat() {
                parser.expect(TT::SemiColon);
            } else {
                parser.expect(TT::SemiColon)?;
            };

            let ast = RAst::new(Ast::TypePublic { name, expr })?;
            (Desc::Stat, ast)
        } else if let Some(op) = _expect_overload_operator_def(parser)? {
            let paramets = _paramets_list(parser)?;

            comment(parser)?;

            let (_, body) = block_expr(parser)?;

            let ast = RAst::new(Ast::OverloadDef {
                op: op,
                paramets,
                body,
            })?;
            (Desc::StatExpr, ast)
        } else {
            stat(parser, false)?
        };

        comment(parser)?;

        stats.push(ast).map_err(|_| Error::OutOfMemory)?;
    }

    parser.expect(TT::RBrace)?;

    let ast = RAst::new(Ast::TypeDef {
        name: name.clone(),
        stats,
    })?;

    Ok((Desc::StatExpr, ast, name))
}

fn if_(parser: &mut Parser, must_expr: bool) -> PResult {
    let if_pos = parser.current_pos();

    parser.expect(TT::If)?;
    parser.expect(TT::LPar)?;

    // cond
    let (_, cond) = expr(parser)?;

    parser.expect(TT::RPar)?;

    comment(parser)?;

    // true body
    let (true_desc, truebody) = _stat_or_expr(parser)?;

    comment(parser)?;

    // false body
    let (false_desc, falsebody) = if let Ok(_) = parser.expect(TT::Else) {
        comment(parser)?;

        let (fd, falsebody) = _stat_or_expr(parser)?;
        (Some(fd), Some(falsebody))
    } else {
        (None, None)
    };

    if let Some(false_desc) = &false_desc {
        if must_expr && !true_desc.is_expr() {
            return Err(parse_error_fmt!(if_pos, "true-body must be expressions"));
        } else if must_expr && !false_desc.is_expr() {
            return Err(parse_error_fmt!(if_pos, "false-body must be expressions"));
        } else if true_desc.is_stat() != false_desc.is_stat() {
            return Err(parse_error_fmt!(
                if_pos,
                "true-body and false-body must be both expressions or statements"
            ));
        }
    }

    let body_desc = false_desc.unwrap_or(true_desc);

    let desc = if body_desc.is_var_expr() {
        Desc::Expr
    } else {
        body_desc
    };

    let ast = RAst::new(Ast::If {
        is_expr: body_desc.is_expr(),
        cond,
        truebody,
        falsebody,
    })?;
    Ok((desc, ast))
}

fn while_(parser: &mut Parser, must_expr: bool) -> PResult {
    parser.expect(TT::While)?;
    parser.expect(TT::LPar)?;

    // cond
    let (_, cond) = expr(parser)?;

    parser.expect(TT::RPar)?;

    comment(parser)?;

    // body
    let (body_desc, body) = _stat_or_expr(parser)?;

    if must_expr && !body_desc.is_expr() {
        return Err(parse_error_fmt!(
            parser.current_pos(),
            "while-body must be expressions"
        ));
    }

    let desc = if body_desc.is_var_expr() {
        Desc::Expr
    } else {
        body_desc
    };

    let ast = RAst::new(Ast::While {
        is_expr: desc.is_expr(),
        cond,
        body,
    })?;

    Ok((desc, ast))
}

fn for_(parser: &mut Parser, must_expr: bool) -> PResult {
    parser.expect(TT::For)?;
    parser.expect(TT::LPar)?;

    let name_tk = parser.expect(TT::Ident)?;

    parser.expect(TT::Colon)?;

    let (_, expr) = expr(parser)?;

    parser.expect(TT::RPar)?;

    comment(parser)?;

    // body
    let (body_desc, body) = _stat_or_expr(parser)?;

    if must_expr && !body_desc.is_expr() {
        return Err(parse_error_fmt!(
            parser.current_pos(),
            "for-body must be expressions"
        ));
    }

    let desc = if body_desc.is_var_expr() {
        Desc::Expr
    } else {
        body_desc
    };

    let name = RString::new(name_tk.source())?;

    let ast = RAst::new(Ast::For {
        is_expr: desc.is_expr(),
        name,
        expr,
        body,
    })?;

    Ok((desc, ast))
}

fn _expect_binop<'c, 's>(parser: &mut Parser<'s>, level: usize) -> Result<Token<'s>, Error> {
    match level {
        11 => parser.expect_any(&[TT::Or]),
        10 => parser.expect_any(&[TT::And]),
        9 => parser.expect_any(&[TT::Eq, TT::Ne]),
        8 => parser.expect_any(&[TT::Lt, TT::Gt, TT::Le, TT::Ge]),
        7 => parser.expect_any(&[TT::Cmp]),
        6 => parser.expect_any(&[TT::BitOr]),
        5 => parser.expect_any(&[TT::BitXor]),
        4 => parser.expect_any(&[TT::BitAnd]),
        3 => parser.expect_any(&[TT::Shl, TT::Shr]),
        2 => parser.expect_any(&[TT::Add, TT::Minus]),
        1 => parser.expect_any(&[TT::Star, TT::Div, TT::IDiv, TT::Mod]),
        _ => {
            return Err(parse_error_fmt!(
                parser.current_pos(),
                "invalid priority level"
            ))
        }
    }
}

const MAX_BINOP_LEVEL: usize = 11;
fn binary_expr(parser: &mut Parser, level: usize) -> PResult {
    if level == 0 {
        return pow_expr(parser);
    }

    let (mut left_desc, mut left) = binary_expr(parser, level - 1)?;

    while let Ok(tk) = _expect_binop(parser, level) {
        let right_desc = if let Some(op) = _tk_to_arith_op(tk) {
            let (right_desc, right) = binary_expr(parser, level - 1)?;
            left = RAst::new(Ast::ArithExpr { op, left, right })?;
            right_desc
        } else if let Some(op) = _tk_to_cmp_op(tk) {
            let (right_desc, right) = binary_expr(parser, level - 1)?;
            left = RAst::new(Ast::CmpExpr { op, left, right })?;
            right_desc
        } else {
            return Err(parse_error_fmt!(
                parser.current_pos(),
                "expected arithmetic operator or comparison operator, but give \"{:?}\"",
                tk.source()
            ));
        };

        left_desc = if right_desc.is_var_expr() {
            Desc::Expr
        } else {
            right_desc
        };
    }

    Ok((left_desc, left))
}

fn pow_expr(parser: &mut Parser) -> PResult {
    let (left_desc, left) = unary_expr(parser)?;

    if parser.match_(TT::Pow) {
        parser.next_token()?;
        let (right_desc, right) = pow_expr(parser)?;

        let op = ArithOp::Pow;
        let ast = RAst::new(Ast::ArithExpr { op, left, right })?;
        let desc = if right_desc.is_var_expr() {
            Desc::Expr
        } else {
            right_desc
        };
        Ok((desc, ast))
    } else {
        Ok((left_desc, left))
    }
}

fn unary_expr(parser: &mut Parser) -> PResult {
    if parser.match_any(&[TT::Not, TT::BitNot]) {
        let op_tk = unsafe { parser.next_token()?.unwrap_unchecked() };
        let (expr_desc, expr) = unary_expr(parser)?;

        let op = match op_tk.token_type() {
            TT::Not => UnaryOp::Not,
            TT::BitNot => UnaryOp::BitNot,
            _ => return Err(runtime_error_fmt!("invalid UnaryOp")),
        };

        let desc = if expr_desc.is_var_expr() {
            Desc::Expr
        } else {
            expr_desc
        };
        let ast = RAst::new(Ast::UnaryExpr { op, expr })?;

        Ok((desc, ast))
    } else {
        atom(parser)
    }
}

fn atom(parser: &mut Parser) -> PResult {
    let (desc, ast) = if parser.match_(TT::Int) {
        let tk = parser.expect(TT::Int)?;
        let n = tk
            .as_int()
            .ok_or_else(|| runtime_error_fmt!("invalid int literal: {}", tk.source()))?;
        let ast = RAst::new(Ast::Int(n as Int))?;
        (Desc::Expr, ast)
    } else if parser.match_(TT::Float) {
        let tk = parser.expect(TT::Float)?;
        let n = tk
            .as_float()
            .ok_or_else(|| runtime_error_fmt!("invalid float literal: {}", tk.source()))?;
        let ast = RAst::new(Ast::Float(n as Float))?;
        (Desc::Expr, ast)
    } else if parser.match_(TT::String) {
        string_literal(parser)?
    } else if _match_tuple_constructor(parser) {
        tuple_constructor(parser)?
    } else if _match_map_constructor(parser) {
        map_constructor(parser)?
    } else if parser.match_(TT::LBrack) {
        array_constructor(parser)?
    } else if parser.match_(TT::LBrace) {
        block_expr(parser)?
    } else {
        prefix_expr(parser)?
    };

    Ok((desc, ast))
}

fn _args_list(parser: &mut Parser) -> Result<Array<Ref<RAst>>, Error> {
    if parser.match_(TT::LPar) {
        parser.next_token()?;

        let mut args = Array::new(allocator());
        if !parser.match_(TT::RPar) {
            let (_, ast) = expr(parser)?;
            args.push(ast).map_err(|_| Error::OutOfMemory)?;
            while parser.expect(TT::Comma).is_ok() {
                let (_, ast) = expr(parser)?;
                args.push(ast).map_err(|_| Error::OutOfMemory)?;
            }
        }
        parser.expect(TT::RPar)?;
        Ok(args)
    } else {
        Err(parse_error_fmt!(
            parser.current_pos(),
            "expect arguments list"
        ))
    }
}

fn _prefix_expr(parser: &mut Parser, prefix_desc: Desc, prefix: Ref<RAst>) -> PResult {
    let (desc, ast) = if parser.match_(TT::LPar) {
        // 函数调用 => a(...)
        let args = _args_list(parser)?;
        let ast = RAst::new(Ast::Call { func: prefix, args })?;
        (Desc::Expr, ast)
    } else if parser.match_all(&[TT::Dot, TT::Ident, TT::LPar]) {
        // 调用对象的方法 => a.b(...)
        parser.expect(TT::Dot)?;
        let name_tk = parser.expect(TT::Ident)?;
        let name = RString::new(name_tk.source())?;
        let args = _args_list(parser)?;
        let ast = RAst::new(Ast::MethodCall {
            target: prefix,
            name,
            args,
        })?;
        (Desc::Expr, ast)
    } else if parser.match_all(&[TT::DbColon, TT::Ident, TT::LPar]) {
        // 调用对象的属性 => a::b(...)
        parser.expect(TT::DbColon)?;
        let name_tk = parser.expect(TT::Ident)?;
        let name = RString::new(name_tk.source())?;
        let args = _args_list(parser)?;
        let ast = RAst::new(Ast::AttrCall {
            target: prefix,
            name,
            args,
        })?;
        (Desc::Expr, ast)
    } else if parser.match_(TT::LBrack) {
        // 获取对象的下标 => a[b]
        parser.expect(TT::LBrack)?;
        let (_, idx_ast) = expr(parser)?;
        parser.expect(TT::RBrack)?;

        let ast = RAst::new(Ast::Index {
            expr: prefix,
            index: idx_ast,
        })?;
        (Desc::VarExpr, ast)
    } else if parser.match_(TT::Dot) {
        // 获取对象属性 => a.b
        parser.next_token()?;
        let name_tk = parser.expect(TT::Ident)?;
        let name = RString::new(name_tk.source())?;
        let ast = RAst::new(Ast::Attr { expr: prefix, name })?;
        (Desc::VarExpr, ast)
    } else if parser.match_(TT::DbColon) {
        // 获取对象属性 => a::b
        parser.next_token()?;
        let name_tk = parser.expect(TT::Ident)?;
        let name = RString::new(name_tk.source())?;
        let ast = RAst::new(Ast::Attr { expr: prefix, name })?;
        (Desc::VarExpr, ast)
    } else {
        return Ok((prefix_desc, prefix));
    };

    let (desc, ast) = _prefix_expr(parser, desc, ast)?;

    Ok((desc, ast))
}
fn prefix_expr(parser: &mut Parser) -> PResult {
    let (p_desc, p_ast) = if parser.match_(TT::LPar) {
        parser.next_token()?;
        let (_, ast) = expr(parser)?;
        parser.expect(TT::RPar)?;
        (Desc::Expr, ast)
    } else if let Ok(name_tk) = parser.expect(TT::Ident) {
        let name = RString::new(name_tk.source())?;
        let ast = RAst::new(Ast::Ident { name })?;
        (Desc::VarExpr, ast)
    } else {
        return Err(parse_error_fmt!(
            parser.current_pos(),
            r#"expect "( <Expr> )" or "<Ident>""#
        ));
    };
    _prefix_expr(parser, p_desc, p_ast)
}

fn block_expr(parser: &mut Parser) -> PResult {
    parser.expect(TT::LBrace)?;

    let mut stats = Array::new(allocator());
    let mut expr = None;

    comment(parser)?;

    while !parser.match_(TT::RBrace) {
        let (desc, ast) = _stat_or_expr(parser)?;

        comment(parser)?;

        if parser.match_(TT::RBrace) && desc.is_expr() {
            expr = Some(ast);
            break;
        } else {
            stats.push(ast).map_err(|_| Error::OutOfMemory)?;
        }
    }

    parser.expect(TT::RBrace)?;

    let ast = RAst::new(Ast::Block { stats, expr })?;
    Ok((Desc::StatExpr, ast))
}

// TODO: 更准确的判断方式。
fn _match_tuple_constructor(parser: &mut Parser) -> bool {
    let mut stack = 0;
    let mut i = 0;

    match parser.peek_token(0) {
        Ok(Some(tk)) if tk.token_type() == TT::LPar => (),
        _ => return false,
    }

    // 当()内包含冒号时视为元组。
    while let Ok(Some(tk)) = parser.peek_token(i) {
        if [TT::LPar, TT::LBrace, TT::LBrack].contains(&tk.token_type()) {
            stack += 1;
        } else if [TT::RPar, TT::RBrace, TT::RBrack].contains(&tk.token_type()) {
            stack -= 1;
        } else if tk.token_type() == TT::Comma && stack == 1 {
            return true;
        }
        if stack == 0 {
            break;
        }

        i += 1;
    }
    return false;
}

fn tuple_constructor(parser: &mut Parser) -> PResult {
    let mut expr_asts = Array::new(allocator());

    parser.expect(TT::LPar)?;

    let (_first_expr_desc, first_expr_ast) = expr(parser)?;
    parser.expect(TT::Comma)?;

    expr_asts
        .push(first_expr_ast)
        .map_err(|_| Error::OutOfMemory)?;

    if !parser.match_(TT::RPar) {
        let (_, expr_ast) = expr(parser)?;
        expr_asts.push(expr_ast).map_err(|_| Error::OutOfMemory)?;

        while parser.match_(TT::Comma) {
            parser.next_token()?;
            let (_, expr_ast) = expr(parser)?;
            expr_asts.push(expr_ast).map_err(|_| Error::OutOfMemory)?;
        }
    }

    parser.expect(TT::RPar)?;

    let ast = RAst::new(Ast::Tuple(expr_asts))?;

    Ok((Desc::Expr, ast))
}

fn array_constructor(parser: &mut Parser) -> PResult {
    parser.expect(TT::LBrack)?;
    let mut expr_asts = Array::new(allocator());

    if !parser.match_(TT::RBrack) {
        let (_, expr_ast) = expr(parser)?;
        expr_asts.push(expr_ast).map_err(|_| Error::OutOfMemory)?;

        while parser.match_(TT::Comma) {
            parser.next_token()?;
            let (_, expr_ast) = expr(parser)?;
            expr_asts.push(expr_ast).map_err(|_| Error::OutOfMemory)?;
        }
    }
    parser.expect(TT::RBrack)?;

    let ast = RAst::new(Ast::Array(expr_asts))?;

    Ok((Desc::Expr, ast))
}

// TODO: 更准确的判断方式。
fn _match_map_constructor(parser: &mut Parser) -> bool {
    let mut stack = 0;
    let mut i = 0;

    // {} 表示空的Map
    if parser.match_all(&[TT::LBrace, TT::RBrace]) {
        return true;
    }

    // 当{}内包含冒号时视为Map
    while let Ok(Some(tk)) = parser.peek_token(i) {
        if tk.token_type() == TT::LBrace {
            stack += 1;
        } else if tk.token_type() == TT::RBrace {
            stack -= 1;
        } else if tk.token_type() == TT::Colon && stack == 1 {
            return true;
        }
        if stack == 0 {
            break;
        }

        i += 1;
    }
    return false;
}

fn _map_field(parser: &mut Parser) -> Result<(Ref<RAst>, Ref<RAst>), Error> {
    let (_, key_ast) = if parser.match_(TT::Ident) {
        let tk = parser.expect(TT::Ident)?;
        let k_str = RString::new(tk.source())?;
        (Desc::Expr, RAst::new(Ast::String(k_str))?)
    } else if parser.match_(TT::String) {
        string_literal(parser)?
    } else if parser.match_(TT::LBrack) {
        parser.expect(TT::LBrack)?;
        let (key_desc, key_ast) = expr(parser)?;
        parser.expect(TT::RBrack)?;
        (key_desc, key_ast)
    } else {
        return Err(parse_error_fmt!(parser.current_pos(), "invalid map key"));
    };
    // let (_, key_ast) = expr(parser)?;
    parser.expect(TT::Colon)?;
    let (_, value_ast) = expr(parser)?;

    Ok((key_ast, value_ast))
}
fn map_constructor(parser: &mut Parser) -> PResult {
    parser.expect(TT::LBrace)?;

    let mut kv_asts = Array::new(allocator());

    if !parser.match_(TT::RBrace) {
        let (key_ast, value_ast) = _map_field(parser)?;
        kv_asts
            .push((key_ast, value_ast))
            .map_err(|_| Error::OutOfMemory)?;

        while parser.match_(TT::Comma) {
            parser.next_token()?;
            let (key_ast, value_ast) = _map_field(parser)?;
            kv_asts
                .push((key_ast, value_ast))
                .map_err(|_| Error::OutOfMemory)?;
        }
    }

    parser.expect(TT::RBrace)?;

    let ast = RAst::new(Ast::Map(kv_asts))?;

    Ok((Desc::StatExpr, ast))
}

fn string_literal(parser: &mut Parser) -> PResult {
    if let Ok(str_tk) = parser.expect(TT::String) {
        let mut buf_arr = Array::new(allocator());
        buf_arr
            .resize(str_tk.source().len(), 0)
            .map_err(|_| Error::new_outofmemory())?;

        let buf = buf_arr.as_slice_mut();
        if let Some(s) = str_tk.as_string(buf) {
            let ast = RAst::new(Ast::String(RString::new(s)?))?;
            Ok((Desc::Expr, ast))
        } else {
            Err(parse_error_fmt!(
                parser.current_pos(),
                "can not convert {:?} to string",
                str_tk.token_type(),
            ))
        }
    } else {
        Err(parse_error_fmt!(
            parser.current_pos(),
            "expect string literal"
        ))
    }
}
