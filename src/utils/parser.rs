use anyhow::{Context, Error};
use std::error::Error as StdError;
use std::str::FromStr;

pub fn parse_next_field<'a, T, Iter>(iter: &mut Iter) -> Result<T, Error>
where
    Iter: Iterator<Item = &'a str>,
    T: FromStr,
    T::Err: Send + Sync + StdError + 'static,
{
    maybe_parse_next_field(iter)?.context("Expected field, but absent")
}

pub fn parse_next_field_opt<'a, T, Iter>(iter: &mut Iter) -> Result<T, Error>
where
    Iter: Iterator<Item = &'a str>,
    T: FromStr + Default,
    T::Err: Send + Sync + StdError + 'static,
{
    maybe_parse_next_field(iter)
        .transpose()
        .unwrap_or(Ok(Default::default()))
}

pub fn maybe_parse_next_field<'a, T, Iter>(iter: &mut Iter) -> Result<Option<T>, Error>
where
    Iter: Iterator<Item = &'a str>,
    T: FromStr,
    T::Err: Send + Sync + StdError + 'static,
{
    iter.next()
        .map(|s| T::from_str(s).context("Failed to parse field"))
        .transpose()
}
