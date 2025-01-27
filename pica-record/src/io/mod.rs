//! Utilities for reading and writing PICA+ records.

use std::io;

use bstr::ByteSlice;
use thiserror::Error;

use crate::parser::LF;
use crate::{ByteRecord, ParsePicaError};

type ParseResult<'a> = Result<ByteRecord<'a>, ParsePicaError>;
type ReadResult<T> = Result<T, ReadPicaError>;

mod reader;
mod writer;

pub use reader::{Reader, ReaderBuilder, RecordsIterator};
pub use writer::{
    ByteRecordWrite, GzipWriter, PlainWriter, WriterBuilder,
};

/// An error that can occur when reading PICA+ records from a
/// [BufReader](std::io::BufReader).
#[derive(Error, Debug)]
pub enum ReadPicaError {
    #[error("parse error")]
    Parse(#[from] ParsePicaError),

    #[error("io error")]
    Io(#[from] io::Error),
}

impl ReadPicaError {
    /// Returns true, if the underlying error was caused by parsing an
    /// invalid record.
    pub fn is_invalid_record(&self) -> bool {
        matches!(self, Self::Parse(ParsePicaError::InvalidRecord(_)))
    }
}

/// An extension of [BufRead](`std::io::BufRead`) which provides a
/// convenience API for reading [ByteRecord](`crate::ByteRecord`)s.
pub trait BufReadExt: io::BufRead {
    /// Executes the given closure on each parsed line in the underlying
    /// reader.
    ///
    /// If the underlying reader or the closure returns an error, then
    /// the iteration stops and the error is returned. If the closure
    /// returns `false` the iteration is stopped and no error is
    /// returned.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::io::{Cursor, Seek};
    ///
    /// use pica_record::io::BufReadExt;
    ///
    /// # fn main() { example().unwrap(); }
    /// fn example() -> anyhow::Result<()> {
    ///     let mut reader =
    ///         Cursor::new(b"003@ \x1f0abc\x1e\n003@ \x1f0def\x1e\n");
    ///
    ///     // iterate over all records
    ///     let mut count = 0;
    ///     reader.for_pica_record(|result| {
    ///         let _record = result?;
    ///         count += 1;
    ///         Ok(true)
    ///     })?;
    ///
    ///     assert_eq!(count, 2);
    ///
    ///     // stop iteration after first record
    ///     reader.rewind()?;
    ///     count = 0;
    ///     reader.for_pica_record(|result| {
    ///         let _record = result?;
    ///         count += 1;
    ///         Ok(false)
    ///     })?;
    ///
    ///     assert_eq!(count, 1);
    ///
    ///     Ok(())
    /// }
    /// ```
    fn for_pica_record<F>(&mut self, mut f: F) -> ReadResult<()>
    where
        F: FnMut(ParseResult) -> ReadResult<bool>,
    {
        // The following code is based on the `io::BufReadExt` trait of
        // the `bstr` crate. It was necessary to duplicate the code, in
        // order to use a different result type.
        // https://docs.rs/bstr/1.0.1/src/bstr/io.rs.html#289-341

        let mut bytes = vec![];
        let mut res = Ok(());
        let mut consumed = 0;

        'outer: loop {
            {
                let mut buf = self.fill_buf()?;

                while let Some(index) = buf.find_byte(LF) {
                    let (line, rest) = buf.split_at(index + 1);
                    buf = rest;
                    consumed += line.len();

                    let result = ByteRecord::from_bytes(line);
                    match f(result) {
                        Ok(false) => break 'outer,
                        Err(err) => {
                            res = Err(err);
                            break 'outer;
                        }
                        _ => (),
                    }
                }

                bytes.extend_from_slice(buf);
                consumed += buf.len();
            }

            self.consume(consumed);
            consumed = 0;

            self.read_until(LF, &mut bytes)?;
            if bytes.is_empty() {
                break;
            }

            let result = ByteRecord::from_bytes(&bytes);
            match f(result) {
                Ok(false) => break,
                Err(err) => {
                    res = Err(err);
                    break;
                }
                _ => (),
            }

            bytes.clear();
        }

        self.consume(consumed);
        res
    }
}

impl<B: io::BufRead> BufReadExt for B {}
