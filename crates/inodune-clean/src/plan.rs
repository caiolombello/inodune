//! Short-lived, backend-owned authorization for a previously scanned cache.
//!
//! The serializable view is presentation data only. The authority lives in this
//! non-serializable value and is consumed by value immediately before execution.
use inodune_core::{
    scan::{Fingerprint, ScanSnapshot},
    Risk, SupportedAction,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    os::unix::ffi::OsStrExt,
    path::{Component, Path},
    sync::Arc,
    time::{Duration, Instant},
};

const PLAN_TTL: Duration = Duration::from_secs(120);
const PLAN_RULE_VERSION: &[u8] = b"inodune-cache-plan-v1";
const MAX_PLAN_ITEMS: usize = 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Trash,
    Permanent,
}

#[derive(Clone, Debug, Serialize)]
pub struct PlanItem {
    pub candidate_id: String,
    pub display_path: String,
    pub logical_bytes: u64,
    pub allocated_bytes_estimate: Option<u64>,
}

/// This structure may cross IPC. It never carries filesystem authority.
#[derive(Clone, Debug, Serialize)]
pub struct PlanView {
    pub id: String,
    pub hash: String,
    pub snapshot_id: String,
    pub action: Action,
    pub expires_in_seconds: u64,
    pub items: Vec<PlanItem>,
    pub logical_bytes_estimate: u64,
    pub allocated_bytes_estimate: Option<u64>,
}

pub struct PlanSession {
    view: PlanView,
    expires_at: Instant,
    snapshot: Arc<ScanSnapshot>,
    ids: Vec<String>,
}

/// Non-serializable execution authority. Its fields are visible only inside
/// this crate; dropping it invalidates the plan.
pub struct AuthorizedPlan {
    pub(crate) snapshot: Arc<ScanSnapshot>,
    pub(crate) ids: Vec<String>,
    pub(crate) action: Action,
    pub(crate) view: PlanView,
}

impl PlanSession {
    pub fn prepare(
        snapshot: Arc<ScanSnapshot>,
        requested_ids: &[String],
        action: Action,
    ) -> Result<Self, String> {
        Self::prepare_until(snapshot, requested_ids, action, Instant::now() + PLAN_TTL)
    }

    fn prepare_until(
        snapshot: Arc<ScanSnapshot>,
        requested_ids: &[String],
        action: Action,
        expires_at: Instant,
    ) -> Result<Self, String> {
        if !snapshot.complete || snapshot.cancelled {
            return Err("A análise está incompleta ou cancelada; faça uma nova análise.".into());
        }

        let mut by_id = std::collections::HashMap::with_capacity(snapshot.candidates.len());
        for candidate in &snapshot.candidates {
            if by_id.insert(candidate.id.as_str(), candidate).is_some() {
                return Err("A análise contém identificadores repetidos.".into());
            }
        }

        let mut ids = Vec::with_capacity(requested_ids.len().min(MAX_PLAN_ITEMS));
        let mut seen_ids = HashSet::new();
        let mut seen_coverage = HashSet::new();
        for id in requested_ids {
            if !seen_ids.insert(id.as_str()) {
                continue;
            }
            if ids.len() >= MAX_PLAN_ITEMS {
                return Err("O plano excede o limite de 1.000 itens.".into());
            }
            let candidate = by_id
                .get(id.as_str())
                .ok_or_else(|| "O item não pertence à análise atual.".to_owned())?;
            if candidate.risk != Risk::Low
                || !matches!(candidate.supported_action, SupportedAction::CacheContents)
                || !matches!(
                    candidate.detector_id.as_str(),
                    "python.pycache" | "thumbnails"
                )
            {
                return Err("O item está protegido ou ainda não tem uma ação aprovada.".into());
            }
            let coverage = candidate
                .coverage_key
                .as_deref()
                .ok_or_else(|| "A cobertura do item é incerta.".to_owned())?;
            if !seen_coverage.insert(coverage) {
                return Err("O plano contém itens com cobertura sobreposta.".into());
            }
            let target = snapshot
                .targets
                .get(id)
                .ok_or_else(|| "O manifesto do item está ausente.".to_owned())?;
            if !target.complete || target.files.is_empty() {
                return Err("O manifesto do item está incompleto.".into());
            }
            validate_relative(&target.relative)?;
            let absolute = target.root.path().join(&target.relative);
            if absolute.to_str().is_none() || candidate.display_path != absolute.to_string_lossy() {
                return Err("O caminho do item não pode ser representado sem ambiguidade.".into());
            }
            for (path, _) in &target.files {
                validate_relative(path)?;
                if path.strip_prefix(&target.relative).is_err() || path == &target.relative {
                    return Err("O manifesto contém um caminho fora do cache aprovado.".into());
                }
                if path.to_str().is_none() || target.root.path().join(path).to_str().is_none() {
                    return Err(
                        "O manifesto contém um caminho não Unicode; item preservado.".into(),
                    );
                }
            }
            ids.push(id.clone());
        }
        if ids.is_empty() {
            return Err("Selecione ao menos um item elegível.".into());
        }

        let mut items = Vec::with_capacity(ids.len());
        let mut logical_bytes_estimate = 0u64;
        let mut allocated_total = Some(0u64);
        for id in &ids {
            let candidate = by_id[id.as_str()];
            logical_bytes_estimate = logical_bytes_estimate
                .checked_add(candidate.logical_bytes)
                .ok_or_else(|| "Estimativa de bytes excede o limite.".to_owned())?;
            allocated_total = match (allocated_total, candidate.allocated_bytes_estimate) {
                (Some(total), Some(size)) => Some(
                    total
                        .checked_add(size)
                        .ok_or_else(|| "Estimativa de espaço excede o limite.".to_owned())?,
                ),
                _ => None,
            };
            items.push(PlanItem {
                candidate_id: candidate.id.clone(),
                display_path: candidate.display_path.clone(),
                logical_bytes: candidate.logical_bytes,
                allocated_bytes_estimate: candidate.allocated_bytes_estimate,
            });
        }

        let id = random_plan_id()?;
        let hash = plan_hash(&snapshot, &ids, action)?;
        let remaining = expires_at.saturating_duration_since(Instant::now());
        let view = PlanView {
            id,
            hash,
            snapshot_id: snapshot.id.clone(),
            action,
            expires_in_seconds: remaining.as_secs(),
            items,
            logical_bytes_estimate,
            allocated_bytes_estimate: allocated_total,
        };
        Ok(Self {
            view,
            expires_at,
            snapshot,
            ids,
        })
    }

    pub fn view(&self) -> PlanView {
        let mut view = self.view.clone();
        view.expires_in_seconds = self
            .expires_at
            .saturating_duration_since(Instant::now())
            .as_secs();
        view
    }

    /// Consume a locally held plan exactly once. A renderer-provided JSON object
    /// can never be deserialized into `PlanSession` or `AuthorizedPlan`.
    pub fn consume(self, expected_hash: &str) -> Result<AuthorizedPlan, String> {
        if Instant::now() >= self.expires_at {
            return Err("O plano expirou; faça uma nova análise.".into());
        }
        if !constant_time_equal(self.view.hash.as_bytes(), expected_hash.as_bytes()) {
            return Err("O plano foi alterado; faça uma nova revisão.".into());
        }
        Ok(AuthorizedPlan {
            snapshot: self.snapshot,
            ids: self.ids,
            action: self.view.action,
            view: self.view,
        })
    }
}

fn validate_relative(path: &Path) -> Result<(), String> {
    if path
        .to_str()
        .is_some_and(|s| s.chars().any(char::is_control))
        || path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|part| {
            matches!(
                part,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
        || path.as_os_str().as_bytes().contains(&0)
    {
        return Err("O manifesto contém caminho relativo inválido.".into());
    }
    Ok(())
}

fn random_plan_id() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|error| format!("Não foi possível criar plano seguro: {error}"))?;
    Ok(format!("plan-{}", hex(&bytes)))
}

fn plan_hash(snapshot: &ScanSnapshot, ids: &[String], action: Action) -> Result<String, String> {
    let mut hash = Sha256::new();
    hash_field(&mut hash, PLAN_RULE_VERSION);
    hash_field(&mut hash, snapshot.id.as_bytes());
    hash_field(
        &mut hash,
        match action {
            Action::Trash => b"trash",
            Action::Permanent => b"permanent",
        },
    );
    for id in ids {
        let candidate = snapshot
            .candidates
            .iter()
            .find(|candidate| candidate.id == *id)
            .ok_or_else(|| "O item não pertence à análise atual.".to_owned())?;
        let target = snapshot
            .targets
            .get(id)
            .ok_or_else(|| "O manifesto do item está ausente.".to_owned())?;
        hash_field(&mut hash, id.as_bytes());
        hash_field(&mut hash, candidate.detector_id.as_bytes());
        hash_field(&mut hash, candidate.detector_version.as_bytes());
        hash_field(&mut hash, candidate.display_path.as_bytes());
        hash_field(&mut hash, &candidate.logical_bytes.to_le_bytes());
        match candidate.allocated_bytes_estimate {
            Some(bytes) => {
                hash_field(&mut hash, b"allocated-known");
                hash_field(&mut hash, &bytes.to_le_bytes());
            }
            None => hash_field(&mut hash, b"allocated-unknown"),
        }
        if let Some(coverage) = &candidate.coverage_key {
            hash_field(&mut hash, coverage.as_bytes());
        } else {
            hash_field(&mut hash, b"coverage-unknown");
        }
        hash_field(&mut hash, target.root.path().as_os_str().as_bytes());
        hash_field(&mut hash, target.relative.as_os_str().as_bytes());
        hash_fingerprint(&mut hash, &target.fingerprint);
        hash_field(&mut hash, &(target.files.len() as u64).to_le_bytes());
        for (path, fingerprint) in &target.files {
            hash_field(&mut hash, path.as_os_str().as_bytes());
            hash_fingerprint(&mut hash, fingerprint);
        }
    }
    Ok(hex(&hash.finalize()))
}

fn hash_fingerprint(hash: &mut Sha256, value: &Fingerprint) {
    for number in [value.dev, value.ino, value.size, value.blocks, value.nlink] {
        hash_field(hash, &number.to_le_bytes());
    }
    hash_field(hash, &value.mode.to_le_bytes());
    hash_field(hash, &value.mtime.to_le_bytes());
    hash_field(hash, &value.mtime_nsec.to_le_bytes());
    hash_field(hash, &value.ctime.to_le_bytes());
    hash_field(hash, &value.ctime_nsec.to_le_bytes());
    hash_field(hash, &value.uid.to_le_bytes());
}

fn hash_field(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u64).to_le_bytes());
    hash.update(bytes);
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    output
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use inodune_core::scan::{scan, ApprovedRoot, ScanControl, ScanLimits, ScopeKind};
    use std::fs;

    struct Fixture {
        _dir: tempfile::TempDir,
        snapshot: Arc<ScanSnapshot>,
    }

    impl Fixture {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("projects");
            let cache = root.join("sample/__pycache__");
            fs::create_dir_all(&cache).unwrap();
            fs::write(root.join("sample/mod.py"), b"pass\n").unwrap();
            let mut pyc = vec![0u8; 64];
            pyc[..4].copy_from_slice(&[0xa7, 0x0d, 13, 10]);
            pyc[16] = b'c';
            fs::write(cache.join("mod.cpython-311.pyc"), pyc).unwrap();
            let root = ApprovedRoot::open(&root, ScopeKind::Projects).unwrap();
            let snapshot = scan(
                &[root],
                &ScanControl::default(),
                &ScanLimits::default(),
                |_| {},
            )
            .unwrap();
            assert!(snapshot.complete);
            assert_eq!(snapshot.candidates.len(), 1);
            Self {
                _dir: dir,
                snapshot: Arc::new(snapshot),
            }
        }
        fn prepare(&self) -> PlanSession {
            PlanSession::prepare(
                self.snapshot.clone(),
                &[self.snapshot.candidates[0].id.clone()],
                Action::Trash,
            )
            .unwrap()
        }
    }

    #[test]
    fn plan_is_one_use_and_binds_action_inventory_and_manifest() {
        let f = Fixture::new();
        let plan = f.prepare();
        let view = plan.view();
        assert_eq!(view.items.len(), 1);
        assert_eq!(view.action, Action::Trash);
        assert!((119..=120).contains(&view.expires_in_seconds));
        assert_eq!(view.logical_bytes_estimate, 64);
        assert!(plan.consume(&view.hash).is_ok());

        let permanent = PlanSession::prepare(
            f.snapshot.clone(),
            &[f.snapshot.candidates[0].id.clone()],
            Action::Permanent,
        )
        .unwrap();
        assert_ne!(permanent.view().hash, view.hash);
        assert!(permanent.consume("00").is_err());
    }

    #[test]
    fn rejects_protected_unknown_empty_and_incomplete_snapshots() {
        let f = Fixture::new();
        assert!(PlanSession::prepare(f.snapshot.clone(), &[], Action::Trash).is_err());
        assert!(
            PlanSession::prepare(f.snapshot.clone(), &["forged".into()], Action::Trash).is_err()
        );
        let mut partial = (*f.snapshot).clone();
        partial.complete = false;
        assert!(PlanSession::prepare(
            Arc::new(partial),
            &[f.snapshot.candidates[0].id.clone()],
            Action::Trash
        )
        .is_err());
        let mut protected = (*f.snapshot).clone();
        protected.candidates[0].risk = Risk::Protected;
        let protected = Arc::new(protected);
        assert!(PlanSession::prepare(
            protected,
            &[f.snapshot.candidates[0].id.clone()],
            Action::Permanent
        )
        .is_err());
    }

    #[test]
    fn rejects_overlapping_and_non_unicode_paths() {
        let f = Fixture::new();
        let mut duplicate = (*f.snapshot).clone();
        let mut second = duplicate.candidates[0].clone();
        second.id.push_str("-copy");
        duplicate.targets.insert(
            second.id.clone(),
            duplicate.targets[&duplicate.candidates[0].id].clone(),
        );
        second.coverage_key = duplicate.candidates[0].coverage_key.clone();
        duplicate.candidates.push(second);
        assert!(PlanSession::prepare(
            Arc::new(duplicate),
            &[
                f.snapshot.candidates[0].id.clone(),
                format!("{}-copy", f.snapshot.candidates[0].id)
            ],
            Action::Trash
        )
        .is_err());

        use std::os::unix::ffi::OsStringExt;
        let mut non_unicode = (*f.snapshot).clone();
        let id = non_unicode.candidates[0].id.clone();
        let target = non_unicode.targets.get_mut(&id).unwrap();
        target.files[0].0 = target
            .relative
            .join(std::ffi::OsString::from_vec(vec![b'x', 0xff]));
        assert!(PlanSession::prepare(Arc::new(non_unicode), &[id], Action::Trash).is_err());
    }

    #[test]
    fn expired_plan_cannot_be_consumed() {
        let f = Fixture::new();
        let plan = PlanSession::prepare_until(
            f.snapshot.clone(),
            &[f.snapshot.candidates[0].id.clone()],
            Action::Trash,
            Instant::now() - Duration::from_secs(1),
        )
        .unwrap();
        let hash = plan.view.hash.clone();
        assert!(plan.consume(&hash).is_err());
    }
}
