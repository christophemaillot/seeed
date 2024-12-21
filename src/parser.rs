use std::fmt::Display;
use chumsky::{Parser, prelude::*,};
use chumsky::text::{ident};
use serde::{Serialize, Serializer};

pub struct Script {
    pub items: Vec<ScriptItem>,
}

impl From<Vec<ScriptItem>> for Script {
    fn from(items: Vec<ScriptItem>) -> Self {
        Self { items }
    }
}

#[derive(Debug)]
pub enum ScriptItem {
    RemoteSingle(String),
    Remote(Vec<String>),
    FnCall(String, Vec<Expr>),
    VarAssign(String, Expr),
    Comment(),
    EmptyLine(),
}

#[derive(Debug, Clone)]
pub enum Expr {
    String(String),
    HereDoc(String),
    Variable(String),
}

impl Serialize for Expr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        match self {
            Self::String(s) => serializer.serialize_str(s),
            Self::HereDoc(s) => serializer.serialize_str(s),
            Self::Variable(s) => serializer.serialize_str(s),
        }
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::String(s) => {
                write!(f, "{}", s)
            }
            Expr::HereDoc(s) => {
                write!(f, "{}", s)
            }
            Expr::Variable(s) => {
                write!(f, "${}", s)
            }
        }
    }
}

// define the grammar for the script
pub fn script_parser() -> impl Parser<char, Script, Error = Simple<char>> {

    let whitespace = one_of(" \t");
    let whitespaces = one_of(" \t").repeated();

    let comment = just("#")
        .ignore_then(whitespace.clone().repeated())
        .ignore_then(
            just("\n").not().repeated().collect::<String>()
        )
        .then_ignore(just("\n"))
        .map(|_| ScriptItem::Comment());


    let single_remote_line = whitespace.clone().repeated().ignored()
        .then_ignore(just("|"))
        .then(just("\n").not().repeated().collect::<String>())
        .then_ignore(just("\n"))
        .map(|(_, s) | ScriptItem::RemoteSingle(s))
        .boxed();

    let multi_remote_line = just("+")
        .then_ignore(just("\n").not().repeated())
        .then_ignore(just("\n"))

        .then(
            single_remote_line.clone().repeated().map(|v|
                v.into_iter().map(|i| match i {
                    ScriptItem::RemoteSingle(s) => s,
                    _ => unreachable!()
                }).collect()
            )
        )
        .then_ignore( just("+"))
        .then_ignore(just("\n").not().repeated())
        .then_ignore(just("\n"))
        .map(|(_, s)| ScriptItem::Remote(s))
        .boxed();

    let empty_line = whitespace.clone().repeated().then_ignore(just("\n")).map(|_| ScriptItem::EmptyLine());


    /*
    let escape = just('\\')
        .then(choice((
            just('\\'),
            just('/'),
            just('"'),
            just('b').to('\x08'),
            just('f').to('\x0C'),
            just('n').to('\n'),
            just('r').to('\r'),
            just('t').to('\t'),
        )))
        .ignored()
        .boxed();

     */

    let string_exp = none_of("\\\"")
//        .or(escape)
        .repeated()
        .delimited_by(just('"'), just('"'))
        .map(|v| v.iter().collect())
        .map(Expr::String);

    let heredoc_exp = just("<<<").ignored()
        .then(
            ident()
                .then_with(|i|
                    just(format!("{}>>>", i)).not()
                        .repeated()
                        .then_ignore(just(format!("{}>>>", i)))
                )
        )
        .map(|(_,c)| Expr::HereDoc(c.into_iter().collect()))
        .boxed();

    let variable_exp = ident().map(Expr::Variable);

    let expr = choice(
        (string_exp, heredoc_exp, variable_exp),
    );

    let fn_call = ident()
        .then_ignore(just("("))
        .then(expr.clone().separated_by(just(",").padded()))
        .then_ignore(just(")"))
        .map(|(name, args)| ScriptItem::FnCall(name.to_string(), args))
        .boxed();

    let var_assign = just("let").ignored()
        .then_ignore(whitespaces.clone())
        .then(ident())
        .then_ignore(whitespaces.clone())
        .then_ignore(just("="))
        .then_ignore(whitespaces.clone())
        .then(expr.clone())
        .then_ignore(whitespaces.clone())
        .then_ignore(just("\n"))
        .map(|((_, name), exp)| ScriptItem::VarAssign(name, exp))
        .boxed();

    let script_item = choice((
        comment,
        empty_line,
        multi_remote_line,
        single_remote_line,
        var_assign,
        fn_call
    ));

    let script = script_item.repeated().map(Script::from); //.then_ignore(end())

    script
}