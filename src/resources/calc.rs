use anyhow::{Error, Result};
use serde::Deserialize;
use smallvec::SmallVec;
use smartstring::alias::String;
use std::{collections::VecDeque, fmt, str::FromStr};

use crate::{
    bonuses::Modifier,
    character::Character,
    resources::{Resource, ResourceRef},
    try_from_str,
};

#[derive(Copy, Clone, Debug)]
pub struct Context<'a> {
    pub character: &'a Character,
    pub rref: &'a ResourceRef,
    pub target: Option<&'a ResourceRef>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Op {
    Add,
    // Subtract,
    // Multiply,
    // Divide,
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Add => write!(f, "+"),
            // Self::Subtract => write!(f, "-"),
            // Self::Multiply => write!(f, "*"),
            // Self::Divide => write!(f, "/"),
        }
    }
}

impl Op {
    fn is_associative(self) -> bool {
        match self {
            Self::Add => true,
            // Self::Subtract => false,
            // Self::Multiply => true,
            // Self::Divide => false,
        }
    }

    fn apply(self, x: Modifier, y: Modifier) -> Modifier {
        match self {
            Self::Add => x + y,
            // Self::Subtract => x - y,
            // Self::Multiply => x * y,
            // Self::Divide => x / y,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(try_from = "smartstring::alias::String")]
pub enum Calculation {
    Named(String),
    Choice(String),
    Modifier(Modifier),
    Op(Op, Vec<Calculation>),
}

impl Default for Calculation {
    fn default() -> Self {
        Self::Modifier(Modifier::new())
    }
}

impl Calculation {
    pub fn evaluate(&self, ctx: Context<'_>) -> Modifier {
        match self {
            Self::Named(name) => ctx.character.get_modifier(name),
            Self::Choice(choice) => {
                let resource = match ctx.rref.resource() {
                    None => {
                        debug!("When attempting to evaluate choice, failed to look up resouce for reference {}", ctx.rref);
                        return Modifier::new();
                    }
                    Some(r) => r,
                };
                match resource.get_choice(choice, ctx) {
                    None => {
                        debug!("When attempting to evaluate a choice, failed to find a numeric value set for resource {}", resource);
                        Modifier::new()
                    }
                    Some(v) => v,
                }
            }
            Self::Modifier(m) => m.clone(),
            Self::Op(op, terms) => {
                let mut iter = terms.iter();
                let mut value = match iter.next() {
                    None => return Modifier::new(),
                    Some(t) => t.evaluate(ctx),
                };
                for term in iter {
                    let next = term.evaluate(ctx);
                    value = op.apply(value, next);
                }
                value
            }
        }
    }
}

try_from_str!(Calculation);

impl FromStr for Calculation {
    type Err = Error;

    fn from_str(s: &str) -> Result<Calculation> {
        Err(anyhow::anyhow!("TODO: parse calculation from {:?}", s))
    }
}

mod parens {
    use super::Calculation;
    use std::fmt;

    pub struct Parens<'a> {
        pub calc: &'a Calculation,
    }

    impl fmt::Display for Parens<'_> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "({})", self.calc)
        }
    }
}

impl fmt::Display for Calculation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Named(name) => write!(f, "{}", name),
            Self::Choice(choice) => write!(f, "${}", choice),
            Self::Modifier(m) => write!(f, "{}", m),
            Self::Op(op, terms) => {
                for (i, t) in terms.iter().enumerate() {
                    let pt;
                    let t: &dyn fmt::Display = match t {
                        Self::Op(_, _) => {
                            pt = parens::Parens { calc: &t };
                            &pt
                        }
                        _ => &t,
                    };
                    if i == 0 {
                        write!(f, "{}", t)?;
                    } else {
                        write!(f, "{} {}", op, t)?;
                    }
                }
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug)]
enum CalculatedStringPart {
    Calc(Calculation),
    Literal(String),
}

impl CalculatedStringPart {
    fn evaluate(&self, s: &mut String, ctx: Context<'_>) -> fmt::Result {
        use std::fmt::Write;

        match self {
            Self::Calc(c) => write!(s, "{}", c.evaluate(ctx)),
            Self::Literal(l) => write!(s, "{}", l),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(try_from = "smartstring::alias::String")]
pub struct CalculatedString {
    parts: SmallVec<[CalculatedStringPart; 3]>,
}

impl CalculatedString {
    pub fn evaluate(&self, character: &Character, mut resource: ResourceRef) -> String {
        let mut s = String::new();
        for part in self.parts.iter() {
            let len_before = s.len();
            let context = Context {
                character,
                rref: &mut resource,
                target: None,
            };
            if let Err(_) = part.evaluate(&mut s, context) {
                s.truncate(len_before);
                s.push_str("<<error>>");
            }
        }
        s
    }
}

impl Default for CalculatedString {
    fn default() -> Self {
        CalculatedString {
            parts: SmallVec::default(),
        }
    }
}

try_from_str!(CalculatedString);

impl FromStr for CalculatedString {
    type Err = Error;

    fn from_str(mut s: &str) -> Result<Self> {
        let mut parts = SmallVec::new();
        let mut in_literal = true;
        while !s.is_empty() {
            if in_literal {
                match s.find("[[") {
                    Some(loc) => {
                        parts.push(CalculatedStringPart::Literal(String::from(&s[..loc])));
                        s = &s[loc + 2..];
                        in_literal = false;
                    }
                    None => {
                        parts.push(CalculatedStringPart::Literal(String::from(s)));
                        s = "";
                    }
                }
            } else {
                match s.find("]]") {
                    Some(loc) => {
                        let calc = s[..loc].parse()?;
                        s = &s[loc + 2..];
                        parts.push(CalculatedStringPart::Calc(calc));
                        in_literal = true;
                    }
                    None => return Err(anyhow::anyhow!("Missing closing ]] in description")),
                }
            }
        }
        Ok(CalculatedString { parts })
    }
}
