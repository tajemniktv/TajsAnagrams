//! Immutable bytes shared by the parsers of one execution, with exact identities.
use crate::{
    contracts::{DataIdentity, DataRepresentation},
    control::Control,
};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::File,
    io::{self, Read},
    path::Path,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LimitScope {
    File,
    Role,
    Aggregate,
}
#[derive(Debug)]
struct CorpusLimitError {
    role: String,
    scope: LimitScope,
    maximum: usize,
}
impl std::fmt::Display for CorpusLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let scope = match self.scope {
            LimitScope::File => "file",
            LimitScope::Role => "per-role",
            LimitScope::Aggregate => "aggregate",
        };
        write!(
            f,
            "corpus role '{}' exceeds its {scope} byte limit of {}",
            self.role, self.maximum
        )
    }
}
impl std::error::Error for CorpusLimitError {}

pub(crate) fn is_file_limit_error(error: &io::Error) -> bool {
    error
        .get_ref()
        .and_then(|context| context.downcast_ref::<CorpusLimitError>())
        .is_some_and(|context| context.scope == LimitScope::File)
}

/// Limits source bytes consumed by corpus roles during one intake phase.
/// Limits are enforced against bytes returned by `Read`, never file metadata.
/// Phrase indexes instead charge their canonical logical JSON stream. This is
/// an input budget, not a bound on parser heap use or SQLite page storage.
pub struct CorpusReadBudget {
    max_role_bytes: Option<usize>,
    max_total_bytes: Option<usize>,
    role_bytes: HashMap<String, usize>,
    total_bytes: usize,
}

impl CorpusReadBudget {
    pub fn new(max_role_bytes: Option<usize>, max_total_bytes: Option<usize>) -> Self {
        Self {
            max_role_bytes,
            max_total_bytes,
            role_bytes: HashMap::new(),
            total_bytes: 0,
        }
    }

    fn role_bytes(&self, role: &str) -> usize {
        self.role_bytes.get(role).copied().unwrap_or(0)
    }

    fn remaining(
        &self,
        role: &str,
        role_file_limit: Option<usize>,
        file_bytes: usize,
    ) -> Option<(usize, LimitScope, usize)> {
        let used = self.role_bytes(role);
        [
            role_file_limit.map(|maximum| {
                (
                    maximum.saturating_sub(file_bytes),
                    LimitScope::File,
                    maximum,
                )
            }),
            self.max_role_bytes
                .map(|maximum| (maximum.saturating_sub(used), LimitScope::Role, maximum)),
            self.max_total_bytes.map(|maximum| {
                (
                    maximum.saturating_sub(self.total_bytes),
                    LimitScope::Aggregate,
                    maximum,
                )
            }),
        ]
        .into_iter()
        .flatten()
        .min_by_key(|(remaining, _, _)| *remaining)
    }

    /// Wrap text sources before buffering, or decompressor output before parsing.
    /// The exclusive borrow keeps
    /// all readers in one intake phase on the same cumulative budget.
    pub fn reader<'a, R: Read>(
        &'a mut self,
        inner: R,
        role: &'a str,
        control: &'a Control,
    ) -> BudgetReader<'a, R> {
        self.reader_with_file_limit(inner, role, control, None)
    }

    fn reader_with_file_limit<'a, R: Read>(
        &'a mut self,
        inner: R,
        role: &'a str,
        control: &'a Control,
        file_limit: Option<usize>,
    ) -> BudgetReader<'a, R> {
        BudgetReader {
            inner,
            budget: self,
            role,
            control,
            file_limit,
            file_bytes: 0,
            exceeded: None,
        }
    }

    fn record(&mut self, role: &str, bytes: usize) -> io::Result<()> {
        let role_bytes = self
            .role_bytes(role)
            .checked_add(bytes)
            .ok_or_else(|| io::Error::other("corpus role byte count exceeds platform limits"))?;
        let total_bytes = self.total_bytes.checked_add(bytes).ok_or_else(|| {
            io::Error::other("aggregate corpus byte count exceeds platform limits")
        })?;
        self.role_bytes.insert(role.to_owned(), role_bytes);
        self.total_bytes = total_bytes;
        Ok(())
    }

    pub(crate) fn ensure_fits(&self, role: &str, bytes: usize) -> io::Result<()> {
        if let Some(maximum) = self.max_role_bytes
            && bytes > maximum.saturating_sub(self.role_bytes(role))
        {
            return Err(Self::limit_error(role, LimitScope::Role, maximum));
        }
        if let Some(maximum) = self.max_total_bytes
            && bytes > maximum.saturating_sub(self.total_bytes)
        {
            return Err(Self::limit_error(role, LimitScope::Aggregate, maximum));
        }
        Ok(())
    }

    pub(crate) fn charge(&mut self, role: &str, bytes: usize) -> io::Result<()> {
        self.ensure_fits(role, bytes)?;
        self.record(role, bytes)
    }

    fn limit_error(role: &str, scope: LimitScope, maximum: usize) -> io::Error {
        io::Error::new(
            io::ErrorKind::FileTooLarge,
            CorpusLimitError {
                role: role.to_owned(),
                scope,
                maximum,
            },
        )
    }
}

pub struct BudgetReader<'a, R> {
    inner: R,
    budget: &'a mut CorpusReadBudget,
    role: &'a str,
    control: &'a Control,
    file_limit: Option<usize>,
    file_bytes: usize,
    exceeded: Option<(LimitScope, usize)>,
}

impl<R: Read> Read for BudgetReader<'_, R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.control.check_io()?;
        if buffer.is_empty() {
            return Ok(0);
        }
        if let Some((scope, maximum)) = self.exceeded {
            return Err(CorpusReadBudget::limit_error(self.role, scope, maximum));
        }
        let limiting = self
            .budget
            .remaining(self.role, self.file_limit, self.file_bytes);
        // One overflow probe byte stays in the caller's fixed buffer. It is
        // never accepted by a parser or retained in a Snapshot.
        let size = limiting.map_or(buffer.len(), |(left, _, _)| {
            left.saturating_add(1).min(buffer.len())
        });
        let n = self.inner.read(&mut buffer[..size])?;
        let accepted = limiting.map_or(n, |(left, _, _)| n.min(left));
        let file_bytes = self
            .file_bytes
            .checked_add(accepted)
            .ok_or_else(|| io::Error::other("corpus file byte count exceeds platform limits"))?;
        self.budget.record(self.role, accepted)?;
        self.file_bytes = file_bytes;
        if let Some((left, scope, maximum)) = limiting
            && n > left
        {
            self.exceeded = Some((scope, maximum));
        }
        // Cancellation during the final read must not publish a successful
        // snapshot or masquerade as a corpus limit failure.
        self.control.check_io()?;
        if let Some((scope, maximum)) = self.exceeded {
            return Err(CorpusReadBudget::limit_error(self.role, scope, maximum));
        }
        Ok(n)
    }
}

impl Default for CorpusReadBudget {
    fn default() -> Self {
        Self::new(
            Some(crate::policy::DEFAULT_MAX_CORPUS_ROLE_BYTES),
            Some(crate::policy::DEFAULT_MAX_TOTAL_CORPUS_BYTES),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_budget_is_finite_for_direct_snapshot_loaders() {
        let budget = CorpusReadBudget::default();
        assert_eq!(
            budget.max_role_bytes,
            Some(crate::policy::DEFAULT_MAX_CORPUS_ROLE_BYTES)
        );
        assert_eq!(
            budget.max_total_bytes,
            Some(crate::policy::DEFAULT_MAX_TOTAL_CORPUS_BYTES)
        );
    }

    #[test]
    fn overflow_does_not_partially_update_counters() {
        let mut budget = CorpusReadBudget::new(None, None);
        budget.total_bytes = usize::MAX;
        assert!(budget.record("dictionary", 1).is_err());
        assert_eq!(budget.total_bytes, usize::MAX);
        assert_eq!(budget.role_bytes("dictionary"), 0);
        budget.total_bytes = 0;
        budget.role_bytes.insert("dictionary".into(), usize::MAX);
        assert!(budget.record("dictionary", 1).is_err());
        assert_eq!(budget.total_bytes, 0);
        assert_eq!(budget.role_bytes("dictionary"), usize::MAX);
    }
}

#[derive(Debug)]
pub struct Snapshot {
    pub(crate) bytes: Vec<u8>,
    pub(crate) identity: DataIdentity,
}
impl Snapshot {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn identity(&self) -> &DataIdentity {
        &self.identity
    }
    pub fn load(path: &Path, role: &str, control: &Control) -> io::Result<Self> {
        Self::load_with_budget(path, role, control, &mut CorpusReadBudget::default())
    }

    /// Load a corpus snapshot while charging its actual bytes to a per-run budget.
    pub fn load_with_budget(
        path: &Path,
        role: &str,
        control: &Control,
        budget: &mut CorpusReadBudget,
    ) -> io::Result<Self> {
        Self::load_with_budget_and_file_limit(path, role, control, budget, None)
    }

    /// As `load_with_budget`, with an additional role-local ceiling for formats
    /// that have their own established size limit (such as learned models).
    pub fn load_with_budget_and_file_limit(
        path: &Path,
        role: &str,
        control: &Control,
        budget: &mut CorpusReadBudget,
        file_limit: Option<usize>,
    ) -> io::Result<Self> {
        control.check_io()?;
        let mut file = budget.reader_with_file_limit(File::open(path)?, role, control, file_limit);
        let mut bytes = Vec::new();
        let mut digest = Sha256::new();
        let mut buffer = [0_u8; 16 * 1024];
        loop {
            control.check_io()?;
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            bytes
                .try_reserve(n)
                .map_err(|error| io::Error::new(io::ErrorKind::OutOfMemory, error))?;
            digest.update(&buffer[..n]);
            bytes.extend_from_slice(&buffer[..n]);
        }
        let identity = DataIdentity {
            role: role.to_owned(),
            representation: DataRepresentation::FileBytes,
            present: true,
            sha256: format!("{:x}", digest.finalize()),
            bytes: bytes.len() as u64,
        };
        Ok(Self { bytes, identity })
    }
    pub fn absent(role: &str) -> DataIdentity {
        DataIdentity {
            role: role.to_owned(),
            representation: DataRepresentation::FileBytes,
            present: false,
            sha256: format!("{:x}", Sha256::digest([])),
            bytes: 0,
        }
    }
}
