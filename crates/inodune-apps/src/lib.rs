//! Read-only installed software inventory. No cleanup candidate or mutation API.
mod command;
mod files;
mod packages;
mod snap_api;
mod steam;
pub mod uninstall;
mod uninstall_flatpak;
mod uninstall_snap;
mod vdf;

pub use packages::{AptDpkg, Flatpak, RpmDnf, Snap};
use serde::Serialize;
use std::path::Path;
pub use steam::Steam;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Rpm,
    Apt,
    FlatpakUser,
    FlatpakSystem,
    Snap,
    Steam,
}
#[derive(Debug, Serialize)]
pub struct UsageSignal {
    pub kind: String,
    pub timestamp: u64,
    pub confidence: String,
}
#[derive(Debug, Serialize)]
pub struct InstalledItem {
    pub source: Source,
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    pub size_bytes: Option<u64>,
    pub install_date: Option<u64>,
    pub signals: Vec<UsageSignal>,
    pub risk: String,
    pub protected: bool,
    pub notes: Vec<String>,
}
impl InstalledItem {
    fn new(source: Source, id: &str, name: &str) -> Self {
        Self {
            source,
            id: id.into(),
            name: name.into(),
            version: None,
            size_bytes: None,
            install_date: None,
            signals: vec![],
            risk: "unknown".into(),
            protected: false,
            notes: vec![],
        }
    }
    fn protect(&mut self) {
        self.protected = true;
        self.risk = "protected".into();
    }
}
#[derive(Debug, Serialize)]
pub struct SourceReport {
    pub source: String,
    pub status: String,
    pub items: Vec<InstalledItem>,
    pub warnings: Vec<String>,
}
#[derive(Debug, Serialize)]
pub struct Inventory {
    pub read_only: bool,
    pub distro: Distro,
    pub sources: Vec<SourceReport>,
}
#[derive(Debug, Default, Serialize)]
pub struct Distro {
    pub id: String,
    pub id_like: Vec<String>,
}
impl Distro {
    pub fn parse(s: &str) -> Self {
        let mut result = Self::default();
        for line in s.lines() {
            if let Some((key, value)) = line.split_once('=') {
                let value = value.trim().trim_matches(['"', '\'']);
                if key == "ID" {
                    result.id = value.into();
                }
                if key == "ID_LIKE" {
                    result.id_like = value.split_whitespace().map(str::to_owned).collect();
                }
            }
        }
        result
    }
    fn family(&self, names: &[&str]) -> bool {
        names.contains(&self.id.as_str())
            || self.id_like.iter().any(|id| names.contains(&id.as_str()))
    }
}
pub trait PackageSource {
    fn name(&self) -> &'static str;
    fn available(&self) -> bool;
    fn list(&self) -> Result<SourceReport, String>;
}
pub fn inventory() -> Result<Inventory, String> {
    static ACTIVE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = ACTIVE
        .try_lock()
        .map_err(|_| "Inventário já está em andamento")?;
    if rustix::process::geteuid().is_root() {
        return Err("Inventário recusa execução como root".into());
    }
    let home = std::env::var_os("HOME").ok_or("HOME ausente")?;
    inventory_at(Path::new(&home))
}
fn inventory_at(home: &Path) -> Result<Inventory, String> {
    let distro = Distro::parse(&std::fs::read_to_string("/etc/os-release").unwrap_or_default());
    let rpm = RpmDnf;
    let apt = AptDpkg;
    let flatpak = Flatpak { home };
    let snap = Snap;
    let steam = Steam { home };
    let mut providers: Vec<&dyn PackageSource> = if distro.family(&["ubuntu", "debian"]) {
        vec![&apt, &rpm]
    } else {
        vec![&rpm, &apt]
    };
    providers.extend([&flatpak as &dyn PackageSource, &snap, &steam]);
    let sources = providers
        .into_iter()
        .map(|source| {
            if !source.available() {
                return report(source.name(), "unavailable", vec![]);
            }
            source.list().unwrap_or_else(|e| SourceReport {
                warnings: vec![e],
                ..report(source.name(), "error", vec![])
            })
        })
        .collect();
    Ok(Inventory {
        read_only: true,
        distro,
        sources,
    })
}
fn report(source: &str, status: &str, items: Vec<InstalledItem>) -> SourceReport {
    SourceReport {
        source: source.into(),
        status: status.into(),
        items,
        warnings: vec![],
    }
}
fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 255
        && s.as_bytes()[0].is_ascii_alphanumeric()
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._+-:".contains(&c))
        && !s.contains("..")
}
fn timestamp(s: &str) -> Option<u64> {
    s.parse::<u64>()
        .ok()
        .filter(|t| *t > 0 && *t <= 253402300799)
}
#[cfg(test)]
mod tests;
