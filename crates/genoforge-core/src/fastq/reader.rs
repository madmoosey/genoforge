//! Streaming FASTQ reader: four lines per record, plain or gzip input.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use flate2::bufread::MultiGzDecoder;

use crate::{Error, Result};

const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];
const BUF_CAPACITY: usize = 1 << 16;

/// Lowest byte of a Phred+33 quality string (`!`, Phred 0).
pub const QUAL_OFFSET: u8 = b'!';
/// Highest byte accepted in a quality string (`~`, Phred 93).
const QUAL_MAX: u8 = b'~';

/// One FASTQ record. Fields are raw bytes exactly as read (no case folding).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Record {
    /// Header line without the leading `@`.
    pub name: Vec<u8>,
    /// Nucleotide sequence.
    pub seq: Vec<u8>,
    /// Phred+33 quality string, same length as `seq`.
    pub qual: Vec<u8>,
}

impl Record {
    /// Read length in bases.
    #[must_use]
    pub fn len(&self) -> usize {
        self.seq.len()
    }

    /// `true` for a zero-length read.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.seq.is_empty()
    }
}

/// Streaming FASTQ reader over any [`BufRead`].
///
/// Validates structure as it goes: `@` header, `+` separator, equal sequence
/// and quality lengths, quality bytes in the Phred+33 range. Reports the
/// 1-based record index on error.
pub struct Reader<R> {
    inner: R,
    line: Vec<u8>,
    records_read: u64,
}

impl Reader<Box<dyn BufRead + Send>> {
    /// Open a `.fastq` or `.fastq.gz` file, sniffing gzip from the magic bytes
    /// rather than the extension.
    ///
    /// # Errors
    /// [`Error::Io`] if the file cannot be opened or its first bytes read.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let io_err = |source| Error::Io {
            path: path.to_path_buf(),
            source,
        };
        let file = File::open(path).map_err(io_err)?;
        let mut buf = BufReader::with_capacity(BUF_CAPACITY, file);
        let is_gzip = buf.fill_buf().map_err(io_err)?.starts_with(&GZIP_MAGIC);
        let inner: Box<dyn BufRead + Send> = if is_gzip {
            Box::new(BufReader::with_capacity(
                BUF_CAPACITY,
                MultiGzDecoder::new(buf),
            ))
        } else {
            Box::new(buf)
        };
        Ok(Self::new(inner))
    }
}

impl<R: BufRead> Reader<R> {
    /// Wrap an existing buffered reader (no gzip sniffing).
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            line: Vec::with_capacity(256),
            records_read: 0,
        }
    }

    /// Records successfully read so far.
    #[must_use]
    pub fn records_read(&self) -> u64 {
        self.records_read
    }

    /// Read the next record into `rec`, reusing its buffers.
    ///
    /// Returns `Ok(false)` at a clean end of file. Blank lines are tolerated
    /// only between records (in practice: trailing newlines).
    ///
    /// # Errors
    /// [`Error::Read`] on an I/O failure; [`Error::Parse`] for a malformed or
    /// truncated record.
    pub fn read_into(&mut self, rec: &mut Record) -> Result<bool> {
        loop {
            if !self.read_line()? {
                return Ok(false);
            }
            if !self.line.is_empty() {
                break;
            }
        }
        if self.line[0] != b'@' {
            return Err(self.parse_err(format!(
                "expected '@' at start of header, found {:?}",
                self.line[0] as char
            )));
        }
        rec.name.clear();
        rec.name.extend_from_slice(&self.line[1..]);

        if !self.read_line()? {
            return Err(self.parse_err("truncated record: missing sequence line"));
        }
        rec.seq.clear();
        rec.seq.extend_from_slice(&self.line);

        if !self.read_line()? {
            return Err(self.parse_err("truncated record: missing '+' line"));
        }
        if self.line.first() != Some(&b'+') {
            return Err(self.parse_err(format!(
                "expected '+' separator, found {:?}",
                String::from_utf8_lossy(&self.line)
            )));
        }

        if !self.read_line()? {
            return Err(self.parse_err("truncated record: missing quality line"));
        }
        rec.qual.clear();
        rec.qual.extend_from_slice(&self.line);

        if rec.qual.len() != rec.seq.len() {
            return Err(self.parse_err(format!(
                "quality length {} != sequence length {}",
                rec.qual.len(),
                rec.seq.len()
            )));
        }
        if let Some(&b) = rec
            .qual
            .iter()
            .find(|&&b| !(QUAL_OFFSET..=QUAL_MAX).contains(&b))
        {
            return Err(self.parse_err(format!("quality byte {b:#04x} outside the Phred+33 range")));
        }

        self.records_read += 1;
        Ok(true)
    }

    /// Consume the reader as an iterator of owned records.
    pub fn records(self) -> Records<R> {
        Records { reader: self }
    }

    /// Read one line into `self.line` without the trailing `\n` / `\r\n`.
    /// Returns `Ok(false)` at end of input.
    fn read_line(&mut self) -> Result<bool> {
        self.line.clear();
        let n = self
            .inner
            .read_until(b'\n', &mut self.line)
            .map_err(Error::Read)?;
        if n == 0 {
            return Ok(false);
        }
        if self.line.last() == Some(&b'\n') {
            self.line.pop();
            if self.line.last() == Some(&b'\r') {
                self.line.pop();
            }
        }
        Ok(true)
    }

    fn parse_err(&self, message: impl Into<String>) -> Error {
        Error::Parse {
            format: "FASTQ",
            record: self.records_read + 1,
            message: message.into(),
        }
    }
}

/// Owned-record iterator returned by [`Reader::records`].
pub struct Records<R> {
    reader: Reader<R>,
}

impl<R: BufRead> Iterator for Records<R> {
    type Item = Result<Record>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut rec = Record::default();
        match self.reader.read_into(&mut rec) {
            Ok(true) => Some(Ok(rec)),
            Ok(false) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn parse(bytes: &[u8]) -> Result<Vec<Record>> {
        Reader::new(Cursor::new(bytes)).records().collect()
    }

    fn rec(name: &str, seq: &str, qual: &str) -> Record {
        Record {
            name: name.into(),
            seq: seq.into(),
            qual: qual.into(),
        }
    }

    #[test]
    fn parses_two_records() {
        let got = parse(b"@r1 desc\nACGT\n+\nIIII\n@r2\nGG\n+r2\n!!\n").unwrap();
        assert_eq!(
            got,
            vec![rec("r1 desc", "ACGT", "IIII"), rec("r2", "GG", "!!")]
        );
    }

    #[test]
    fn handles_crlf_missing_final_newline_and_trailing_blank_lines() {
        let got = parse(b"@r1\r\nAC\r\n+\r\nII\r\n\n\n").unwrap();
        assert_eq!(got, vec![rec("r1", "AC", "II")]);
        let got = parse(b"@r1\nAC\n+\nII").unwrap();
        assert_eq!(got, vec![rec("r1", "AC", "II")]);
    }

    #[test]
    fn empty_input_yields_nothing() {
        assert_eq!(parse(b"").unwrap(), Vec::<Record>::new());
        assert_eq!(parse(b"\n\n").unwrap(), Vec::<Record>::new());
    }

    #[test]
    fn zero_length_read_is_valid() {
        assert_eq!(parse(b"@e\n\n+\n\n").unwrap(), vec![rec("e", "", "")]);
    }

    #[test]
    fn reports_record_index_on_errors() {
        let cases: [(&[u8], &str); 5] = [
            (b"@r1\nAC\n+\nII\nr2\nAC\n+\nII\n", "expected '@'"),
            (b"@r1\nAC\n+\nII\n@r2\nAC\n", "missing '+'"),
            (b"@r1\nAC\n+\nII\n@r2\nAC\n-\nII\n", "expected '+'"),
            (
                b"@r1\nAC\n+\nII\n@r2\nAC\n+\nIII\n",
                "quality length 3 != sequence length 2",
            ),
            (
                b"@r1\nAC\n+\nII\n@r2\nAC\n+\nI\x1f\n",
                "outside the Phred+33 range",
            ),
        ];
        for (input, needle) in cases {
            let err = parse(input).unwrap_err();
            match &err {
                Error::Parse {
                    format,
                    record,
                    message,
                } => {
                    assert_eq!(*format, "FASTQ");
                    assert_eq!(*record, 2, "input {:?}", String::from_utf8_lossy(input));
                    assert!(message.contains(needle), "{message}");
                }
                other => panic!("unexpected error {other:?}"),
            }
        }
    }

    #[test]
    fn truncated_after_header_is_record_one() {
        let err = parse(b"@r1\n").unwrap_err();
        assert!(matches!(err, Error::Parse { record: 1, .. }), "{err}");
    }

    #[test]
    fn from_path_sniffs_gzip() {
        use flate2::{write::GzEncoder, Compression};
        use std::io::Write;

        let dir = std::env::temp_dir();
        let plain = dir.join(format!("genoforge-{}.fastq", std::process::id()));
        let gz = dir.join(format!("genoforge-{}.fastq.gz", std::process::id()));
        let body = b"@r1\nACGT\n+\nIIII\n";
        std::fs::write(&plain, body).unwrap();
        let mut enc = GzEncoder::new(Vec::new(), Compression::fast());
        enc.write_all(body).unwrap();
        std::fs::write(&gz, enc.finish().unwrap()).unwrap();

        for p in [&plain, &gz] {
            let got: Vec<Record> = Reader::from_path(p)
                .unwrap()
                .records()
                .collect::<Result<_>>()
                .unwrap();
            assert_eq!(got, vec![rec("r1", "ACGT", "IIII")], "{}", p.display());
        }
        std::fs::remove_file(plain).ok();
        std::fs::remove_file(gz).ok();
    }

    #[test]
    fn from_path_missing_file_is_io_error() {
        let Err(err) = Reader::from_path("/nonexistent/genoforge.fastq") else {
            panic!("expected an error");
        };
        assert!(matches!(err, Error::Io { .. }), "{err}");
    }
}
