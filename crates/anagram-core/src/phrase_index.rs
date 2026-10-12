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
    digest: Option<&'a mut Sha256>,
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
            if let Some(digest) = self.digest.as_deref_mut() {
                digest.update(chunk);
            }
            self.bytes = total;
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn hash_json_line<T: serde::Serialize>(
    digest: Option<&mut Sha256>,
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

fn install_progress_handler(
    connection: &Connection,
    control: &crate::control::Control,
) -> io::Result<()> {
    let callback_control = control.clone();
    connection
        .progress_handler(1000, Some(move || callback_control.check().is_err()))
        .map_err(error)
}
type Record = (Option<String>, Option<i64>, Option<i64>);
fn read_record(row: &rusqlite::Row<'_>, budget: &CorpusReadBudget) -> io::Result<Record> {
    let text_bytes = row.get::<_, Option<i64>>(3).map_err(error)?.unwrap_or(0);
    let text_bytes = usize::try_from(text_bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "negative phrase text length"))?;
    budget.ensure_fits("phrase_index", text_bytes)?;
    Ok((
        row.get::<_, Option<String>>(0).map_err(error)?,
        row.get::<_, Option<i64>>(1).map_err(error)?,
        row.get::<_, Option<i64>>(2).map_err(error)?,
    ))
}
impl PhraseIndex {
    pub fn open(path: &Path) -> io::Result<Self> {
        Self::open_controlled(path, &crate::control::Control::default())
    }
    pub fn open_controlled(path: &Path, control: &crate::control::Control) -> io::Result<Self> {
        Self::open_controlled_with_budget(path, control, &mut CorpusReadBudget::default())
    }

    /// Admit the canonical logical input in table order before any aggregate or
    /// sorted query. Ranker intake does not need to calculate a discarded hash.
    pub fn open_controlled_with_budget(
        path: &Path,
        control: &crate::control::Control,
        budget: &mut CorpusReadBudget,
    ) -> io::Result<Self> {
        control.check_io()?;
        // Deliberately omit URI parsing: a user-supplied filename is a path.
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(error)?;
        install_progress_handler(&connection, control)?;
        // Admission, provenance and every later query see one read snapshot.
        connection.execute_batch("BEGIN").map_err(error)?;
        let mut index = Self {
            connection,
            max_n: 0,
            control: control.clone(),
        };
        index.max_n = index.scan_with_budget(budget)?;
        Ok(index)
    }

    /// Canonical logical identity, not a hash of SQLite pages or WAL files.
    pub fn identity(&self) -> io::Result<DataIdentity> {
        self.identity_with_budget(&mut CorpusReadBudget::default())
    }

    /// Hash logical rows incrementally while enforcing the execution's corpus
    /// byte budget. A row's source text length is checked before allocating it.
    pub fn identity_with_budget(&self, budget: &mut CorpusReadBudget) -> io::Result<DataIdentity> {
        self.scan_with_budget(budget)?;
        self.identity_of_intake()
    }

    /// Validate all keys without serializing or hashing a discarded identity.
    /// Called after budgeted admission, so even unindexed grouping is only run
    /// over an already admitted immutable snapshot.
    pub fn validate_unique_keys(&self) -> io::Result<()> {
        self.control.check_io()?;
        let duplicate: bool = self
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM ngrams GROUP BY text HAVING COUNT(*) > 1)",
                [],
                |row| row.get(0),
            )
            .map_err(error)?;
        self.control.check_io()?;
        if duplicate {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Duplicate phrase keys have no deterministic lookup identity",
            ));
        }
        Ok(())
    }

    fn schema(&self, budget: &CorpusReadBudget) -> io::Result<String> {
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
        Ok(schema)
    }

    fn scan_with_budget(&self, budget: &mut CorpusReadBudget) -> io::Result<usize> {
        let schema = self.schema(budget)?;
        hash_json_line(None, &("schema", schema), budget, &self.control)?;
        // No ORDER BY or aggregate: rows are admitted incrementally, including
        // JSON escaping/field overhead, before SQLite can materialize a sort.
        let mut statement = self
            .connection
            .prepare("SELECT text, n, count, length(CAST(text AS BLOB)) FROM ngrams")
            .map_err(error)?;
        let mut rows = statement.query([]).map_err(error)?;
        let mut maximum = None;
        while let Some(row) = rows.next().map_err(error)? {
            self.control.check_io()?;
            let record = read_record(row, budget)?;
            hash_json_line(None, &record, budget, &self.control)?;
            if let Some(n) = record.1 {
                maximum = Some(maximum.map_or(n, |old: i64| old.max(n)));
            }
        }
        self.control.check_io()?;
        usize::try_from(maximum.unwrap_or(0))
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "negative phrase order"))
    }

    /// Re-read the admitted immutable snapshot for identity, without charging
    /// its logical input twice. SQLite physical I/O/temporary storage is separate.
    pub(crate) fn identity_of_intake(&self) -> io::Result<DataIdentity> {
        self.validate_unique_keys()?;
        let mut budget = CorpusReadBudget::new(None, None);
        let schema = self.schema(&budget)?;
        let mut statement = self
            .connection
            .prepare("SELECT text, n, count, length(CAST(text AS BLOB)) FROM ngrams ORDER BY text COLLATE BINARY, n, count")
            .map_err(error)?;
        let mut rows = statement.query([]).map_err(error)?;
        let mut digest = Sha256::new();
        // Collations can affect lookup semantics even for identical visible rows.
        let header_bytes = hash_json_line(
            Some(&mut digest),
            &("schema", schema),
            &mut budget,
            &self.control,
        )?;
        let mut bytes = u64::try_from(header_bytes).map_err(io::Error::other)?;
        let mut previous = None;
        while let Some(row) = rows.next().map_err(error)? {
            self.control.check_io()?;
            let record = read_record(row, &budget)?;
            if previous.as_ref().is_some_and(|text| text == &record.0) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Duplicate phrase keys have no deterministic lookup identity",
                ));
            }
            let record_bytes =
                hash_json_line(Some(&mut digest), &record, &mut budget, &self.control)?;
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

#[cfg(test)]
mod progress_tests {
    #[test]
    fn sqlite_vm_handler_interrupts_expired_control_deterministically() {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        let control = crate::control::Control::with_deadline(std::time::Instant::now());
        super::install_progress_handler(&connection, &control).unwrap();
        // Run directly without Rust-side prechecks: only the production VM
        // handler can interrupt this query, independent of machine speed.
        let result = connection.query_row(
            "WITH RECURSIVE seq(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM seq WHERE x<10000) SELECT sum(x) FROM seq",
            [], |row| row.get::<_, i64>(0),
        );
        assert_eq!(
            result.unwrap_err().sqlite_error_code(),
            Some(rusqlite::ErrorCode::OperationInterrupted)
        );
        assert_eq!(control.check(), Err("timed_out"));
    }
}
