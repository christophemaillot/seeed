
use std::str::{from_utf8, FromStr};
use std::char::{decode_utf16, REPLACEMENT_CHARACTER};
use std::fmt::{Debug, Display, Formatter};
use pom::char_class::{alpha, alphanum, hex_digit};
use pom::Error;
use pom::parser::{call, Parser};
use pom::parser::{is_a, none_of,  one_of, seq, sym, list, end};
use serde::{Serialize, Serializer};
use log::error;

#[derive(Debug)]
pub struct ScriptAST {
    pub statements: Vec<Statement>,
}

impl ScriptAST {
    pub fn from(statements: Vec<Statement>) -> Self {
        Self { statements }
    }
}

#[derive(Debug, Clone)]
pub enum Statement {
    Comment(),
    EmptyLine(),
    Assign(String, Expression),
    RemoteSingle(String),
    Remote(Vec<String>),
    FnCall(String, Vec<Expression>),
    ForLoop(String, Expression, Vec<Statement>),
}

#[derive(Debug, Clone)]
pub enum Expression {
    Literal(Literal),
    Variable(String),
    FnCall(String, Vec<Expression>),
    Array(Vec<Expression>),
    HereDoc(String),
}

#[derive(Debug, Clone)]
pub enum Literal {
    HereDoc(String),
    String(String),
    Integer(i64),
    Bool(bool),
    Array(Vec<Literal>),
    Void,
}

impl Display for Literal {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Literal::HereDoc(contents) => {
                write!(formatter, "{}", contents)
            }
            Literal::String(s) => {
                write!(formatter, "{}", s)
            }
            Literal::Integer(i) => {
                write!(formatter, "{}", i)
            }
            Literal::Bool(b) => {
                write!(formatter, "{}", b)
            }
            Literal::Array(_) => {
                write!(formatter, "<<array>>")                                                      // TODO
            }
            Literal::Void => {
                write!(formatter, "void")
            }
        }
    }
}


impl Serialize for Literal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        self.to_string().serialize(serializer)
    }
}

// intro to the pom parser and other references :
//
// https://github.com/J-F-Liu/pom/blob/master/doc/article.md
// https://medium.com/pragmalang/parsing-the-world-with-rust-and-pom-77e0e8b5313d
// https://github.com/J-F-Liu/pom/issues/49


// ┌───────────────────────────────────────────────────────────────────────────────────────────┐ //
// │                             parser extensions                                             │ //
// └───────────────────────────────────────────────────────────────────────────────────────────┘ //

pub fn until<'a>(needle: String) -> Parser<'a, u8, String>
{

    fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|window| window == needle)
    }

    Parser::new(move |input: &'a [u8], start: usize| {

        let pos = find_subsequence(&input[start..], needle.as_bytes());
        match pos {
            None => {
                Err(Error::Incomplete)
            }
            Some(position) => {
                let s = from_utf8(&input[start..position + start ]).map_err(|_| Error::Mismatch { message: "Invalid UTF-8 sequence".to_owned(), position: start })?;
                Ok((s.to_owned(), start + position + 1 + needle.len()))
            }
        }
    })
}



// ┌───────────────────────────────────────────────────────────────────────────────────────────┐ //
// │                             utility parsers                                               │ //
// └───────────────────────────────────────────────────────────────────────────────────────────┘ //

/// spaces
///
/// a sequence of space or tab, repeated (including zero times)
///
fn spaces<'a>() -> Parser<'a, u8, ()> {
    one_of(b" \t").repeat(0..).discard()
}

/// integer
///
/// Parses an integer. The parser allow an integer to start with several 0
/// (0000123 parses a valid integer)
fn integer<'a>() -> Parser<'a, u8, i64> {
    let integer = one_of(b"0123456789").repeat(1..).collect().convert(from_utf8);
    integer.convert(|v| v.parse::<i64>().or_else(Err))
}

/// number
///
/// Parses a number (including floating point numbers)
/// returns a parser that returns a f64 if parsed correctly
fn _number<'a>() -> Parser<'a, u8, f64> {
    let integer = one_of(b"123456789") - one_of(b"0123456789").repeat(0..) | sym(b'0');
    let frac = sym(b'.') + one_of(b"0123456789").repeat(1..);
    let exp = one_of(b"eE") + one_of(b"+-").opt() + one_of(b"0123456789").repeat(1..);
    let number = sym(b'-').opt() + integer + frac.opt() + exp.opt();
    number.collect().convert(from_utf8).convert(f64::from_str)
}

/// string
///
/// parses a string, that included escaped characters like \" or \t and \u<code point> expressions
fn string<'a>() -> Parser<'a, u8, String> {
    let special_char = sym(b'\\') | sym(b'/') | sym(b'"')
        | sym(b'b').map(|_|b'\x08') | sym(b'f').map(|_|b'\x0C')
        | sym(b'n').map(|_|b'\n') | sym(b'r').map(|_|b'\r') | sym(b't').map(|_|b'\t');
    let escape_sequence = sym(b'\\') * special_char;
    let char_string = (none_of(b"\\\"") | escape_sequence).repeat(1..).convert(String::from_utf8);
    let utf16_char = seq(b"\\u") * is_a(hex_digit).repeat(4).convert(String::from_utf8).convert(|digits|u16::from_str_radix(&digits, 16));
    let utf16_string = utf16_char.repeat(1..).map(|chars|decode_utf16(chars).map(|r| r.unwrap_or(REPLACEMENT_CHARACTER)).collect::<String>());
    let string = sym(b'"') * (char_string | utf16_string).repeat(0..) - sym(b'"');
    string.map(|s| s.concat())
}

/// identifier
///
/// parse a generic identifier : starts with an alphanumeric or underscore,
fn identifier<'a>() -> Parser<'a, u8, String> {
    ((is_a(alpha) | sym(b'_')) + (is_a(alphanum) | sym(b'_')).repeat(0..)).map(|(prefix, rest)| {
        let rest_str = String::from_utf8(rest).unwrap_or_else(|_| String::new()); // Should be safe as alphanum is ASCII
        format!("{}{}", prefix as char, rest_str)
    })
}

// ┌───────────────────────────────────────────────────────────────────────────────────────────┐ //
// │                                 literal parsers                                           │ //
// └───────────────────────────────────────────────────────────────────────────────────────────┘ //

fn string_literal<'a>() -> Parser<'a, u8, Literal> {
    string().map(Literal::String)
}

fn integer_literal<'a>() -> Parser<'a, u8, Literal> {
    integer().map(Literal::Integer)
}

fn bool_literal<'a>() -> Parser<'a, u8, Literal> {
    seq(b"true").map(|_| Literal::Bool(true)) | seq(b"false").map(|_| Literal::Bool(false))
}

fn literal<'a>() -> Parser<'a, u8, Literal> {
    string_literal() | integer_literal() | bool_literal()
}

// ┌───────────────────────────────────────────────────────────────────────────────────────────┐ //
// │                              expression parsers                                           │ //
// └───────────────────────────────────────────────────────────────────────────────────────────┘ //

fn literal_expression<'a>() -> Parser<'a, u8, Expression> {
    literal().map(Expression::Literal)
}

fn array_expression<'a>() -> Parser<'a, u8, Expression> {
    let elems = list(call(expression), sym(b',') * spaces());
    (sym(b'[') * spaces() * elems - sym(b']')).map(Expression::Array)
}

fn variable_expression<'a>() -> Parser<'a, u8, Expression> {
    let parser = sym(b'$') * identifier();
    parser.map(Expression::Variable)
}

fn heredoc_expression<'a>() -> Parser<'a, u8, Expression> {

    let doc = (seq(b"<<<") * (is_a(alpha).repeat(1..).collect().convert(from_utf8)))
        >> |start| {
        let close_tag = format!("{}>>>", start);

        until(close_tag)

    };
    doc.map(|doc| {
        Expression::HereDoc(doc.to_owned())
    })
}

fn expression<'a>() -> Parser<'a, u8, Expression> {
    literal_expression() | array_expression() | heredoc_expression() | variable_expression()
}

// ┌───────────────────────────────────────────────────────────────────────────────────────────┐ //
// │                                   statement                                               │ //
// └───────────────────────────────────────────────────────────────────────────────────────────┘ //

fn emptyline_statement<'a>() -> Parser<'a, u8, Statement> {
    let till_end = spaces() + sym(b'\n');
    till_end.map(|_| Statement::EmptyLine())
}

fn comment_statement<'a>() -> Parser<'a, u8, Statement> {
    let till_end = none_of(b"\n").repeat(0..);
    let comment = (sym(b'#') *  till_end.collect().convert(from_utf8).map(|_s| Statement::Comment())) - sym(b'\n');
    comment
}

fn assign_statement<'a>() -> Parser<'a, u8, Statement> {
    let assign = spaces() * seq(b"let") * spaces() * identifier() - spaces() - sym(b'=') - spaces() + expression();
    assign.map(|(ident, expr)| Statement::Assign(ident, expr))
}

fn function_call_statement<'a>() -> Parser<'a, u8, Statement> {
    let parser = identifier() - spaces() - sym(b'(') + list(expression(), sym(b',') + spaces()) - sym(b')');
    parser.map(|(name, args)| Statement::FnCall(name, args))
}

fn single_remote_statement<'a>() -> Parser<'a, u8, Statement> {
    let parser =  (spaces() + sym(b'|')) * none_of(b"\n").repeat(0..).collect() - sym(b'\n');
    parser.convert(from_utf8).map(|v| Statement::RemoteSingle(v.to_owned()))
}

fn multi_remote_statement<'a>() -> Parser<'a, u8, Statement> {

    let block_start = spaces() * sym(b'+') * none_of(b"\n").repeat(0..).collect() - sym(b'\n');
    let block_line =  (spaces() + sym(b'|')) * none_of(b"\n").repeat(0..).collect() - sym(b'\n');
    let block_end = spaces() * sym(b'+') - spaces() - sym(b'\n');

    let parser = block_start * block_line.convert(from_utf8).map(|s| s.to_string()).repeat(0..) - block_end;

    parser.map(Statement::Remote)
}

fn for_loop_statement<'a>() -> Parser<'a, u8, Statement> {
    let loop_start_parser = spaces() * seq(b"for") * spaces() * identifier() - spaces() - seq(b"in") - spaces() + expression() - spaces() - sym(b'{');
    let loop_end_parser = spaces() * sym(b'}');


    let parser = loop_start_parser + call(statement).repeat(0..) - loop_end_parser;

    parser.map(|((name, exp), statements)|  Statement::ForLoop(name, exp, statements))
}


fn statement<'a>() -> Parser<'a, u8, Statement> {
    comment_statement() | emptyline_statement() | assign_statement() | function_call_statement() | single_remote_statement() | multi_remote_statement() | for_loop_statement()
}

// ┌───────────────────────────────────────────────────────────────────────────────────────────┐ //
// │                                script parser                                              │ //
// └───────────────────────────────────────────────────────────────────────────────────────────┘ //

pub fn script_parser<'a>() -> Parser<'a, u8, ScriptAST> {
    let script = statement().repeat(0..) -(sym(b'\n').opt()) - end();
    script.map(ScriptAST::from)
}
