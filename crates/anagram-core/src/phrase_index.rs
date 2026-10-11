//! Read-only SQLite phrase corpus. Each worker owns its connection.
use crate::corpus_ranking::PhraseCorpus;
use crate::{
    contracts::{DataIdentity, DataRepresentation},
    provenance::CorpusReadBudget,
};
use rusqlite::{Connection, OpenFlags, params_from_iter};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    io::{self, Write},
    path::Path,
};

struct DigestWriter<'a> {
    digest: &'a mut Sha256,
    budget: &'a mut CorpusReadBudget,
    control: &'a crate::control::Control,
    bytes: usize,
}

impl Write for DigestWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        for chunk in bytes.chunks(16 * 1024) {
            self.control.check_io()?;
            let total = self
                .bytes
                .checked_add(chunk.len())
                .ok_or_else(|| io::Error::other("Phrase identity size overflow"))?;
            self.budget.charge("phrase_index", chunk.len())?;
            self.digest.update(chunk);
            self.bytes = total;
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn hash_json_line<T: serde::Serialize>(
    digest: &mut Sha256,
    value: &T,
    budget: &mut CorpusReadBudget,
    control: &crate::control::Control,
) -> io::Result<usize> {
    let mut writer = DigestWriter {
        digest,
        budget,
        control,
        bytes: 0,
    };
    serde_json::to_writer(&mut writer, value).map_err(|error| {
        io::Error::new(
            error.io_error_kind().unwrap_or(io::ErrorKind::InvalidData),
            error,
        )
    })?;
    writer.write_all(b"\n")?;
    Ok(writer.bytes)
}

pub struct PhraseIndex {
    connection: Connection,
    max_n: usize,
    control: crate::control::Control,
}
fn error(e: rusqlite::Error) -> io::Error {
    io::Error::other(e)
}
impl PhraseIndex {
    pub fn open(path: &Path) -> io::Result<Self> {
        Self::open_controlled(path, &crate::control::Control::default())
    }
    pub fn open_controlled(path: &Path, control: &crate::control::Control) -> io::Result<Self> {
        control.check_io()?;
        // Deliberately omit URI parsing: a user-supplied filename is a path.
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(error)?;
        let callback_control = control.clone();
        connection
            .progress_handler(1000, Some(move || callback_control.check().is_err()))
            .map_err(error)?;
        // MAX(n), provenance and every later query must see one read snapshot.
        connection.execute_batch("BEGIN").map_err(error)?;
        let max_n: i64 = connection
            .query_row("SELECT COALESCE(MAX(n), 0) FROM ngrams", [], |row| {
                row.get(0)
            })
            .map_err(error)?;
        let max_n = usize::try_from(max_n)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "negative phrase order"))?;
        Ok(Self {
            connection,
            max_n,
            control: control.clone(),
        })
    }

    /// Canonical logical identity, not a hash of SQLite pages or WAL files.
    pub fn identity(&self) -> io::Result<DataIdentity> {
        self.identity_with_budget(&mut CorpusReadBudget::default())
    }

    /// Hash logical rows incrementally while enforcing the execution's corpus
    /// byte budget. A row's source text length is checked before allocating it.
    pub fn identity_with_budget(&self, budget: &mut CorpusReadBudget) -> io::Result<DataIdentity> {
        self.control.check_io()?;
        let mut schema_statement = self.connection.prepare(
            "SELECT type, sql, length(CAST(sql AS BLOB)) FROM sqlite_schema WHERE name='ngrams' COLLATE NOCASE"
        ).map_err(error)?;
        let mut schema_rows = schema_statement.query([]).map_err(error)?;
        let schema_row = schema_rows
            .next()
            .map_err(error)?
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing ngrams table"))?;
        let schema_bytes =
            usize::try_from(schema_row.get::<_, i64>(2).map_err(error)?).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "negative phrase schema length")
            })?;
        budget.ensure_fits("phrase_index", schema_bytes)?;
        let kind: String = schema_row.get(0).map_err(error)?;
        let schema: String = schema_row.get(1).map_err(error)?;
        if kind != "table" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Phrase provenance requires a concrete ngrams table, not a dynamic view",
            ));
        }
        let mut statement = self
            .connection
            .prepare("SELECT text, n, count, length(CAST(text AS BLOB)) FROM ngrams ORDER BY text COLLATE BINARY, n, count")
            .map_err(error)?;
        let mut rows = statement.query([]).map_err(error)?;
        let mut digest = Sha256::new();
        // Collations can affect lookup semantics even for identical visible rows.
        let header_bytes = hash_json_line(&mut digest, &("schema", schema), budget, &self.control)?;
        let mut bytes = u64::try_from(header_bytes).map_err(io::Error::other)?;
        let mut previous = None;
        while let Some(row) = rows.next().map_err(error)? {
            self.control.check_io()?;
            let text_bytes = row.get::<_, Option<i64>>(3).map_err(error)?.unwrap_or(0);
            let text_bytes = usize::try_from(text_bytes).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "negative phrase text length")
            })?;
            budget.ensure_fits("phrase_index", text_bytes)?;
            let record = (
                row.get::<_, Option<String>>(0).map_err(error)?,
                row.get::<_, Option<i64>>(1).map_err(error)?,
                row.get::<_, Option<i64>>(2).map_err(error)?,
            );
            if previous.as_ref().is_some_and(|text| text == &record.0) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Duplicate phrase keys have no deterministic lookup identity",
                ));
            }
            let record_bytes = hash_json_line(&mut digest, &record, budget, &self.control)?;
            bytes = bytes
                .checked_add(u64::try_from(record_bytes).map_err(io::Error::other)?)
                .ok_or_else(|| io::Error::other("Phrase identity size overflow"))?;
            previous = Some(record.0);
        }
        self.control.check_io()?;
        Ok(DataIdentity {
            role: "phrase_index".to_owned(),
            representation: DataRepresentation::PhraseRowsV1,
            present: true,
            sha256: format!("{:x}", digest.finalize()),
            bytes,
        })
    }
}
impl PhraseCorpus for PhraseIndex {
    fn max_n(&self) -> usize {
        self.max_n
    }
    fn counts(&self, phrases: &[String]) -> io::Result<HashMap<String, i64>> {
        self.control.check_io()?;
        let mut seen = HashSet::new();
        let unique: Vec<_> = phrases
            .iter()
            .filter(|p| !p.is_empty() && seen.insert(p.as_str()))
            .collect();
        let mut counts = HashMap::new();
        for batch in unique.chunks(200) {
            self.control.check_io()?;
            let placeholders = vec!["?"; batch.len()].join(",");
            let mut statement = self
                .connection
                .prepare(&format!(
                    "SELECT text, count FROM ngrams WHERE text IN ({placeholders})"
                ))
                .map_err(error)?;
            let matches = statement
                .query_map(params_from_iter(batch), |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                })
                .map_err(error)?;
            for item in matches {
                let (phrase, count) = item.map_err(error)?;
                if counts.insert(phrase, count).is_some() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Duplicate phrase keys",
                    ));
                }
            }
        }
        Ok(counts)
    }
}
