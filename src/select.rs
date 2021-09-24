use crate::filter::{
    parse_field_tag, parse_occurrence_matcher, parse_string,
    parse_subfield_code, parse_subfield_filter, ws, OccurrenceMatcher,
    SubfieldFilter, Tag,
};

use nom::branch::alt;
use nom::character::complete::{char, multispace0};
use nom::combinator::{all_consuming, cut, map, opt};
use nom::multi::separated_list1;
use nom::sequence::{delimited, pair, preceded, terminated, tuple};
use nom::{Finish, IResult};

use bstr::BString;
use std::default::Default;
use std::ops::{Add, Deref, Mul};

#[derive(Debug, PartialEq)]
pub struct FieldSelector {
    pub(crate) tag: Tag,
    pub(crate) occurrence: OccurrenceMatcher,
    pub(crate) filter: Option<SubfieldFilter>,
    pub(crate) subfields: Vec<char>,
}

#[derive(Debug, PartialEq)]
pub enum Selector {
    Field(FieldSelector),
    Value(String),
}

#[derive(Debug, Default)]
pub struct Outcome(pub(crate) Vec<Vec<BString>>);

impl Outcome {
    pub fn from_values(values: Vec<BString>) -> Self {
        Self(vec![values])
    }

    pub fn one() -> Self {
        Self(vec![[BString::from("")].to_vec()])
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Deref for Outcome {
    type Target = Vec<Vec<BString>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Mul for Outcome {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        if self.is_empty() {
            return rhs;
        }

        if rhs.is_empty() {
            return self;
        }

        let mut result: Vec<Vec<BString>> = Vec::new();

        for row_lhs in &self.0 {
            for row in &rhs.0 {
                let mut new_row = row_lhs.clone();
                for col in row {
                    new_row.push(col.to_owned());
                }
                result.push(new_row.clone());
            }
        }

        Self(result)
    }
}

impl Add for Outcome {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut result: Vec<Vec<BString>> = Vec::new();

        for row in &self.0 {
            result.push(row.clone())
        }

        for row in &rhs.0 {
            result.push(row.clone())
        }

        Self(result)
    }
}

impl FieldSelector {
    pub fn new(
        tag: Tag,
        occurrence: OccurrenceMatcher,
        filter: Option<SubfieldFilter>,
        subfields: Vec<char>,
    ) -> Self {
        Self {
            tag,
            occurrence,
            filter,
            subfields,
        }
    }
}

pub struct Selectors(Vec<Selector>);

impl Selectors {
    pub fn decode(s: &str) -> Result<Self, String> {
        match parse_selectors(s).finish() {
            Ok((_, selectors)) => Ok(selectors),
            _ => Err("invalid select statement".to_string()),
        }
    }
}

impl Deref for Selectors {
    type Target = Vec<Selector>;

    fn deref(&self) -> &Vec<Selector> {
        &self.0
    }
}

fn parse_selector(i: &str) -> IResult<&str, Selector> {
    alt((
        map(ws(parse_string), Selector::Value),
        map(
            tuple((
                parse_field_tag,
                parse_occurrence_matcher,
                preceded(char('.'), cut(parse_subfield_code)),
            )),
            |(tag, occurrence, subfield)| {
                Selector::Field(FieldSelector::new(
                    tag,
                    occurrence,
                    None,
                    vec![subfield],
                ))
            },
        ),
        map(
            tuple((
                parse_field_tag,
                parse_occurrence_matcher,
                delimited(
                    ws(char('{')),
                    pair(
                        opt(terminated(parse_subfield_filter, ws(char(',')))),
                        separated_list1(ws(char(',')), ws(parse_subfield_code)),
                    ),
                    ws(char('}')),
                ),
            )),
            |(tag, occurrence, (filter, subfields))| {
                Selector::Field(FieldSelector::new(
                    tag, occurrence, filter, subfields,
                ))
            },
        ),
    ))(i)
}

fn parse_selectors(i: &str) -> IResult<&str, Selectors> {
    all_consuming(map(
        delimited(
            multispace0,
            separated_list1(ws(char(',')), parse_selector),
            multispace0,
        ),
        Selectors,
    ))(i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::ComparisonOp;

    #[test]
    fn test_parse_selector() {
        assert_eq!(
            parse_selector("003@.0"),
            Ok((
                "",
                Selector::Field(FieldSelector::new(
                    Tag::Constant("003@".to_string()),
                    OccurrenceMatcher::None,
                    None,
                    vec!['0']
                ))
            ))
        );

        assert_eq!(
            parse_selector("044H/*{ 9, E ,H}"),
            Ok((
                "",
                Selector::Field(FieldSelector::new(
                    Tag::Constant("044H".to_string()),
                    OccurrenceMatcher::Any,
                    None,
                    vec!['9', 'E', 'H']
                ))
            ))
        );

        assert_eq!(
            parse_selector("044H/*{ E == 'm', 9, E , H }"),
            Ok((
                "",
                Selector::Field(FieldSelector::new(
                    Tag::Constant("044H".to_string()),
                    OccurrenceMatcher::Any,
                    Some(SubfieldFilter::Comparison(
                        vec!['E'],
                        ComparisonOp::Eq,
                        vec!["m".to_string()]
                    )),
                    vec!['9', 'E', 'H']
                ))
            ))
        );

        assert_eq!(
            parse_selector("012A/*.a"),
            Ok((
                "",
                Selector::Field(FieldSelector::new(
                    Tag::Constant("012A".to_string()),
                    OccurrenceMatcher::Any,
                    None,
                    vec!['a']
                ))
            ))
        );

        assert_eq!(
            parse_selector("012A/01.a"),
            Ok((
                "",
                Selector::Field(FieldSelector::new(
                    Tag::Constant("012A".to_string()),
                    OccurrenceMatcher::new("01").unwrap(),
                    None,
                    vec!['a']
                ))
            ))
        );
    }
}
