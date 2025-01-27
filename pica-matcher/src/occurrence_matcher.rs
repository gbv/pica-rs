use std::fmt::Display;

use bstr::BStr;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::char;
use nom::combinator::{
    all_consuming, cut, map, success, value, verify,
};
use nom::sequence::{preceded, separated_pair};
use nom::Finish;
use pica_record::parser::{parse_occurrence_digits, ParseResult};
use pica_record::{Occurrence, OccurrenceMut};

use crate::ParseMatcherError;

/// A matcher that matches against PICA+
/// [Occurrence](`pica_record::Occurrence`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OccurrenceMatcher {
    Any,
    Some(OccurrenceMut),
    Range(OccurrenceMut, OccurrenceMut),
    None,
}

impl OccurrenceMatcher {
    /// Create a new tag matcher.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pica_matcher::TagMatcher;
    /// use pica_record::TagRef;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> anyhow::Result<()> {
    ///     let matcher = TagMatcher::new("003@")?;
    ///     assert_eq!(matcher, TagRef::new("003@"));
    ///
    ///     # assert!(TagMatcher::new("003!").is_err());
    ///     Ok(())
    /// }
    /// ```
    pub fn new<T>(expr: T) -> Result<Self, ParseMatcherError>
    where
        T: AsRef<[u8]> + Display,
    {
        all_consuming(parse_occurrence_matcher)(expr.as_ref())
            .finish()
            .map_err(|_| {
                ParseMatcherError::InvalidOccurrenceMatcher(
                    expr.to_string(),
                )
            })
            .map(|(_, matcher)| matcher)
    }

    /// Returns `true` if the given occurrence matches against the
    /// matcher.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pica_matcher::OccurrenceMatcher;
    /// use pica_record::OccurrenceRef;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> anyhow::Result<()> {
    ///     let matcher = OccurrenceMatcher::new("/01-03")?;
    ///     assert!(matcher.is_match(&OccurrenceRef::new("02")));
    ///     assert!(!matcher.is_match(&OccurrenceRef::new("04")));
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn is_match<T>(&self, occurrence: &Occurrence<T>) -> bool
    where
        T: AsRef<[u8]>,
    {
        match self {
            Self::Any => true,
            Self::None => occurrence == "00",
            Self::Some(rhs) => occurrence == rhs,
            Self::Range(min, max) => {
                (occurrence >= min) && (occurrence <= max)
            }
        }
    }
}

impl<T: AsRef<[u8]>> PartialEq<Occurrence<T>> for OccurrenceMatcher {
    fn eq(&self, other: &Occurrence<T>) -> bool {
        self.is_match(other)
    }
}

impl<T: AsRef<[u8]>> PartialEq<Option<&Occurrence<T>>>
    for OccurrenceMatcher
{
    fn eq(&self, other: &Option<&Occurrence<T>>) -> bool {
        match other {
            Some(occurrence) => self.is_match(occurrence),
            None => matches!(self, Self::Any | Self::None),
        }
    }
}

impl<T: AsRef<[u8]>> PartialEq<OccurrenceMatcher> for Occurrence<T> {
    fn eq(&self, matcher: &OccurrenceMatcher) -> bool {
        matcher.is_match(self)
    }
}

impl From<OccurrenceMut> for OccurrenceMatcher {
    fn from(value: OccurrenceMut) -> Self {
        OccurrenceMatcher::Some(value)
    }
}

#[inline]
fn parse_occurrence_range(i: &[u8]) -> ParseResult<OccurrenceMatcher> {
    map(
        verify(
            separated_pair(
                parse_occurrence_digits,
                char('-'),
                parse_occurrence_digits,
            ),
            |(min, max)| min.len() == max.len() && min < max,
        ),
        |(min, max)| {
            OccurrenceMatcher::Range(
                OccurrenceMut::from_unchecked(min),
                OccurrenceMut::from_unchecked(max),
            )
        },
    )(i)
}

#[inline]
fn parse_occurrence_exact(i: &[u8]) -> ParseResult<OccurrenceMatcher> {
    map(
        verify(parse_occurrence_digits, |x: &BStr| x.to_vec() != b"00"),
        |value| OccurrenceMut::from_unchecked(value).into(),
    )(i)
}

pub fn parse_occurrence_matcher(
    i: &[u8],
) -> ParseResult<OccurrenceMatcher> {
    alt((
        preceded(
            char('/'),
            cut(alt((
                parse_occurrence_range,
                parse_occurrence_exact,
                value(OccurrenceMatcher::None, tag("00")),
                value(OccurrenceMatcher::Any, char('*')),
            ))),
        ),
        success(OccurrenceMatcher::None),
    ))(i)
}

#[cfg(test)]
mod tests {
    use nom_test_helpers::prelude::*;
    use pica_record::OccurrenceRef;

    use super::*;

    #[test]
    fn test_parse_occurrence_matcher() -> anyhow::Result<()> {
        assert_done_and_eq!(
            parse_occurrence_matcher(b"/*"),
            OccurrenceMatcher::Any
        );

        assert_done_and_eq!(
            parse_occurrence_matcher(b"/00"),
            OccurrenceMatcher::None
        );

        assert_done_and_eq!(
            parse_occurrence_matcher(b"/01"),
            OccurrenceMatcher::Some(OccurrenceMut::new("01"))
        );

        assert_done_and_eq!(
            parse_occurrence_matcher(b"/01-03"),
            OccurrenceMatcher::Range(
                OccurrenceMut::new("01"),
                OccurrenceMut::new("03"),
            )
        );

        assert_done_and_eq!(
            parse_occurrence_matcher(b""),
            OccurrenceMatcher::None,
        );

        assert_error!(parse_occurrence_matcher(b"/0A"));
        assert_error!(parse_occurrence_matcher(b"/A"));

        Ok(())
    }

    #[test]
    fn test_is_match() -> anyhow::Result<()> {
        let matcher = OccurrenceMatcher::new("/01")?;
        assert!(!matcher.is_match(&OccurrenceRef::new("00")));
        assert!(matcher.is_match(&OccurrenceRef::new("01")));

        let matcher = OccurrenceMatcher::new("/01-03")?;
        assert!(!matcher.is_match(&OccurrenceRef::new("00")));
        assert!(matcher.is_match(&OccurrenceRef::new("01")));
        assert!(matcher.is_match(&OccurrenceRef::new("02")));
        assert!(matcher.is_match(&OccurrenceRef::new("03")));
        assert!(!matcher.is_match(&OccurrenceRef::new("04")));

        let matcher = OccurrenceMatcher::new("/*")?;
        assert!(matcher.is_match(&OccurrenceRef::new("00")));
        assert!(matcher.is_match(&OccurrenceRef::new("01")));

        let matcher = OccurrenceMatcher::new("/00")?;
        assert!(matcher.is_match(&OccurrenceRef::new("00")));
        assert!(!matcher.is_match(&OccurrenceRef::new("01")));

        Ok(())
    }

    #[test]
    fn test_partial_eq() -> anyhow::Result<()> {
        let matcher = OccurrenceMatcher::new("/01")?;
        assert_ne!(matcher, OccurrenceRef::new("00"));
        assert_eq!(matcher, OccurrenceRef::new("01"));
        assert_ne!(matcher, Option::<OccurrenceRef>::None.as_ref());

        let matcher = OccurrenceMatcher::new("/01-03")?;
        assert_ne!(matcher, OccurrenceRef::new("00"));
        assert_eq!(matcher, OccurrenceRef::new("01"));
        assert_eq!(matcher, OccurrenceRef::new("02"));
        assert_eq!(matcher, OccurrenceRef::new("03"));
        assert_ne!(matcher, OccurrenceRef::new("04"));
        assert_ne!(matcher, Option::<OccurrenceRef>::None.as_ref());

        let matcher = OccurrenceMatcher::new("/*")?;
        assert_eq!(matcher, OccurrenceRef::new("000"));
        assert_eq!(matcher, OccurrenceRef::new("00"));
        assert_eq!(matcher, OccurrenceRef::new("001"));
        assert_eq!(matcher, OccurrenceRef::new("01"));
        assert_eq!(matcher, Option::<OccurrenceRef>::None.as_ref());

        let matcher = OccurrenceMatcher::new("/00")?;
        assert_eq!(matcher, OccurrenceRef::new("00"));
        assert_ne!(matcher, OccurrenceRef::new("01"));
        assert_eq!(matcher, Option::<OccurrenceRef>::None.as_ref());

        Ok(())
    }
}
