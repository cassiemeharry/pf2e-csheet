#[cfg(test)]
use proptest::prelude::*;
#[cfg(test)]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use smartstring::alias::String;
use std::{fmt, str::FromStr};
use thiserror::Error;

use crate::{
    bonuses::{Bonus, Modifier, Penalty},
    character::Character,
    choices::Choice,
    common::ResourceRef,
    storage::ResourceStorage,
};

#[derive(Copy, Clone)]
// #[cfg_attr(test, derive(Arbitrary))]
pub struct CalcContext<'a> {
    pub character: &'a Character,
    pub rref: &'a ResourceRef,
    pub target: Option<&'a ResourceRef>,
    pub resources: &'a dyn ResourceStorage,
}

impl<'a> fmt::Debug for CalcContext<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CalcContext")
            .field("character", self.character)
            .field("rref", self.rref)
            .field("target", &self.target)
            .field("resources", &format_args!("…"))
            .finish()
    }
}

impl<'a> CalcContext<'a> {
    pub fn new(
        character: &'a Character,
        rref: &'a ResourceRef,
        resources: &'a dyn ResourceStorage,
    ) -> Self {
        Self {
            character,
            rref,
            target: None,
            resources,
        }
    }

    pub fn with_target(mut self, target: &'a ResourceRef) -> Self {
        self.target = Some(target);
        self
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
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
    // fn is_associative(self) -> bool {
    //     match self {
    //         Self::Add => true,
    //         // Self::Subtract => false,
    //         // Self::Multiply => true,
    //         // Self::Divide => false,
    //     }
    // }

    fn apply(self, x: i16, y: i16) -> i16 {
        match self {
            Self::Add => x + y,
            // Self::Subtract => x - y,
            // Self::Multiply => x * y,
            // Self::Divide => x / y,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(try_from = "smartstring::alias::String")]
pub enum Calculation {
    Named(String),
    Choice(Choice),
    Modifier(Modifier),
    Op(Op, Vec<Calculation>),
}

#[cfg(test)]
impl Arbitrary for Calculation {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: ()) -> Self::Strategy {
        let leaf = prop_oneof![
            prop::string::string_regex("[a-zA-Z][a-zA-Z_]+")
                .unwrap()
                .prop_map_into()
                .prop_map(Calculation::Named),
            any::<Choice>().prop_map(Calculation::Choice),
            any::<Modifier>().prop_map(Calculation::Modifier),
        ];
        leaf.prop_recursive(
            3,  // levels deep
            20, // maximum node count
            5,  // items per collection
            |inner| {
                (any::<Op>(), prop::collection::vec(inner.clone(), 2..10))
                    .prop_map(|(op, calcs)| Self::Op(op, calcs).normalized())
            },
        )
        .boxed()
    }
}

impl Default for Calculation {
    fn default() -> Self {
        Self::Modifier(Modifier::new())
    }
}

impl Calculation {
    fn normalize(&mut self) {
        if let Self::Op(outer_op, outer_terms) = self {
            trace!("Normalizing Op({:?}, {:?})", outer_op, outer_terms);
            if outer_terms.iter().all(|t| matches!(t, Self::Modifier(_))) {
                {
                    trace!(
                        "Normalizing Op({:?}, {:?}) by combining modifiers",
                        outer_op,
                        outer_terms
                    );
                }
                let mut total = Modifier::new();
                match outer_op {
                    Op::Add => {
                        for term in outer_terms.drain(..) {
                            match term {
                                Self::Modifier(m) => total += m,
                                _ => unreachable!(),
                            }
                        }
                    }
                }
                *self = Self::Modifier(total);
                return;
            }
            // Flatten tree with matching ops
            let mut terms = vec![];
            std::mem::swap(&mut terms, outer_terms);
            let mut flattened = true;
            let mut iterations = 0;
            while flattened {
                trace!("Flattening op terms {:?}", terms);
                flattened = false;
                terms.sort_by_key(|term| match term {
                    Self::Op(_, _) => 0,
                    Self::Modifier(_) => 1,
                    Self::Named(_) => 2,
                    Self::Choice(_) => 3,
                });
                let mut new_terms = Vec::with_capacity(terms.len());
                for mut term in terms.drain(..) {
                    term.normalize();
                    match term {
                        Self::Op(inner_op, inner_terms) if *outer_op == inner_op => {
                            flattened = true;
                            new_terms.extend(inner_terms);
                        }
                        Self::Modifier(m1) => match (*outer_op, new_terms.pop()) {
                            (Op::Add, Some(Self::Modifier(m2))) => {
                                new_terms.push(Self::Modifier(m1 + m2));
                                flattened = true;
                            }
                            (Op::Add, Some(other)) => {
                                new_terms.push(other);
                                new_terms.push(Self::Modifier(m1));
                            }
                            (Op::Add, None) => {
                                new_terms.push(Self::Modifier(m1));
                            }
                        },
                        other_term => new_terms.push(other_term),
                    }
                }
                terms = new_terms;
                iterations += 1;
                assert!(iterations < 20);
            }
            trace!("After flatten, terms = {:?}", terms);
            *self = Self::Op(*outer_op, terms);
        }
    }

    pub fn normalized(mut self) -> Self {
        self.normalize();
        self
    }

    // pub fn build_op(left: Self, right: Self, op: Op) -> Self {
    //     let (op, items) = match (left, right) {
    //         (Self::Op(l_op, mut l_terms), Self::Op(r_op, r_terms)) if l_op == op && r_op == op => {
    //             l_terms.extend(r_terms);
    //             (op, l_terms)
    //         }
    //         (Self::Op(l_op, mut terms), other) if l_op == op => {
    //             terms.push(other);
    //             (op, terms)
    //         }
    //         (s, Self::Op(r_op, mut terms)) if r_op == op => {
    //             terms.insert(0, s);
    //             (op, terms)
    //         }
    //         (left, right) => (op, vec![left, right]),
    //     };
    //     // HACK: modifiers with multiple parts deserialize into adding up the
    //     // individual components, so we need to undo that for the roundtrip
    //     // tests to pass.
    //     if items.iter().all(|item| matches!(item, Self::Modifier(_))) {
    //         trace!("In Calculation modifier sum hack");
    //         let mut total = Modifier::new();
    //         match op {
    //             Op::Add => {
    //                 for item in items {
    //                     match item {
    //                         Self::Modifier(m) => total += m,
    //                         _ => unreachable!(),
    //                     }
    //                 }
    //             }
    //         }
    //         Self::Modifier(total)
    //     } else {
    //         Self::Op(op, items)
    //     }
    // }

    pub fn from_number(n: i16) -> Self {
        let m = if n >= 0 {
            Bonus::untyped(n).into()
        } else {
            Penalty::untyped(-n).into()
        };
        Calculation::Modifier(m)
    }

    pub fn evaluate(&self, ctx: CalcContext<'_>) -> i16 {
        match self {
            Self::Named(name) => ctx
                .character
                .get_modifier(name, ctx.target, ctx.resources)
                .total(),
            Self::Choice(choice) => {
                let resource = match ctx.resources.lookup_immediate(ctx.rref) {
                    None => {
                        debug!("When attempting to evaluate choice, failed to look up resouce for reference {}", ctx.rref);
                        return 0;
                    }
                    Some(r) => r,
                };
                match resource.common().get_choice(choice, ctx) {
                    None => {
                        debug!("When attempting to evaluate a choice, failed to find a numeric value set for resource {}", resource);
                        0
                    }
                    Some(v) => v,
                }
            }
            Self::Modifier(m) => m.total(),
            Self::Op(op, terms) => {
                let mut iter = terms.iter();
                let mut value = match iter.next() {
                    None => return 0,
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

#[derive(Debug, Error)]
#[error("Failed to parse calculation")]
pub struct CalculationFromStrError;

impl FromStr for Calculation {
    type Err = CalculationFromStrError;

    fn from_str(s: &str) -> Result<Calculation, Self::Err> {
        trace!("Parsing calculation from {:?}", s);
        match crate::parsers::calculation(s) {
            Ok(c) => {
                trace!("Got calculation {:?}", c);
                Ok(c)
            }
            Err(e) => {
                error!("Failed to parse calculation from {:?}:\n{}", s, e);
                Err(CalculationFromStrError)
            }
        }
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
        trace!("Displaying calculation {:?}", self);
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
                        write!(f, " {} {}", op, t)?;
                    }
                }
                Ok(())
            }
        }
    }
}

impl Serialize for Calculation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let calc = format!("{}", self);
        serializer.serialize_str(calc.as_str())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum CalculatedStringPart {
    Calc(Calculation),
    Literal(
        #[cfg_attr(
            test,
            proptest(
                strategy = "\"..+\".prop_filter_map(\"invalid calculated string literal\", proptest_bad_calc_str_part_literal)"
            )
        )]
        String,
    ),
}

#[cfg(test)]
fn proptest_bad_calc_str_part_literal(s: std::string::String) -> Option<String> {
    if s.contains("[") {
        None
    } else if s.contains("]") {
        None
    } else {
        Some(s.into())
    }
}

impl CalculatedStringPart {
    fn evaluate(&self, s: &mut String, ctx: CalcContext<'_>) -> fmt::Result {
        use std::fmt::Write;

        match self {
            Self::Calc(c) => write!(s, "{}", c.evaluate(ctx)),
            Self::Literal(l) => write!(s, "{}", l),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(try_from = "smartstring::alias::String")]
pub struct CalculatedString {
    parts: SmallVec<[CalculatedStringPart; 3]>,
}

#[cfg(test)]
impl Arbitrary for CalculatedString {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: ()) -> Self::Strategy {
        proptest::collection::vec(any::<CalculatedStringPart>(), 1..=5)
            .prop_map(|parts_vec| {
                let mut parts = SmallVec::new();
                for part in parts_vec {
                    match (parts.pop(), part) {
                        (
                            Some(CalculatedStringPart::Literal(mut l1)),
                            CalculatedStringPart::Literal(l2),
                        ) => {
                            l1.push_str(&l2);
                            parts.push(CalculatedStringPart::Literal(l1));
                        }
                        (
                            Some(l @ CalculatedStringPart::Literal(_)),
                            c @ CalculatedStringPart::Calc(_),
                        ) => {
                            parts.push(l);
                            parts.push(c);
                        }
                        (Some(CalculatedStringPart::Calc(c1)), CalculatedStringPart::Calc(c2)) => {
                            let calc = Calculation::Op(Op::Add, vec![c1, c2]).normalized();
                            parts.push(CalculatedStringPart::Calc(calc));
                        }
                        (
                            Some(c @ CalculatedStringPart::Calc(_)),
                            l @ CalculatedStringPart::Literal(_),
                        ) => {
                            parts.push(c);
                            parts.push(l);
                        }
                        (None, first) => {
                            parts.push(first);
                        }
                    }
                }
                Self { parts }
            })
            .boxed()
    }
}

impl CalculatedString {
    pub fn concat(mut self, other: Self) -> Self {
        use CalculatedStringPart::*;

        for part in other.parts {
            match (self.parts.pop(), part) {
                (None, part) => self.parts.push(part),
                (Some(Calc(c1)), Calc(c2)) => {
                    let mut c = Calculation::Op(Op::Add, vec![c1, c2]);
                    c.normalize();
                    self.parts.push(Calc(c));
                }
                (Some(Literal(mut l1)), Literal(l2)) => {
                    l1.push_str(&l2);
                    self.parts.push(Literal(l1));
                }
                (Some(left @ Calc(_)), right @ Literal(_))
                | (Some(left @ Literal(_)), right @ Calc(_)) => {
                    self.parts.push(left);
                    self.parts.push(right);
                }
            }
        }
        self
    }

    pub fn join_with_literal(mut self, other: Self, part: &str) -> Self {
        self.parts.push(CalculatedStringPart::Literal(part.into()));
        self.parts.extend(other.parts);
        self
    }

    pub fn join_with_calc(mut self, other: Self, part: Calculation) -> Self {
        self.parts.push(CalculatedStringPart::Calc(part.into()));
        self.parts.extend(other.parts);
        self
    }

    pub fn evaluate(
        &self,
        character: &Character,
        rref: &ResourceRef,
        resources: &dyn ResourceStorage,
    ) -> String {
        let mut s = String::new();
        for part in self.parts.iter() {
            let len_before = s.len();
            let context = CalcContext::new(character, rref, resources);
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

impl fmt::Display for CalculatedString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for part in self.parts.iter() {
            match part {
                CalculatedStringPart::Calc(c) => write!(f, "[[ {} ]]", c)?,
                CalculatedStringPart::Literal(s) => write!(f, "{}", s)?,
            }
        }
        Ok(())
    }
}

try_from_str!(CalculatedString);

#[derive(Debug, Error)]
#[error("Failed to parse calculated string")]
pub struct CalculatedStringFromStrError;

impl FromStr for CalculatedString {
    type Err = CalculatedStringFromStrError;

    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        let mut parts = SmallVec::new();
        let mut in_literal = true;
        while !s.is_empty() {
            if in_literal {
                match s.find("[[") {
                    Some(0) => {
                        s = &s[2..];
                        in_literal = false;
                    }
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
                        let calc = s[..loc].parse().map_err(|_| CalculatedStringFromStrError)?;
                        s = &s[loc + 2..];
                        parts.push(CalculatedStringPart::Calc(calc));
                        in_literal = true;
                    }
                    None => return Err(CalculatedStringFromStrError),
                }
            }
        }
        Ok(CalculatedString { parts })
    }
}

serialize_display!(CalculatedString);
