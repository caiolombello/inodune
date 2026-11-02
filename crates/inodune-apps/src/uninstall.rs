//! Desinstalação separada do inventário. Nunca eleva o processo nem recebe shell.
use crate::Source;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub source: Source,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub ids: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Ord, PartialOrd)]
#[serde(deny_unknown_fields)]
pub struct Removal {
    pub id: String,
    pub version: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub id: String,
    pub items: Vec<Removal>,
    pub expires_in_seconds: u64,
    pub steam_handoff: bool,
}
pub trait Transport {
    fn simulate(&mut self, request: &Request) -> Result<Vec<Removal>, String>;
    fn execute(&mut self, request: &Request, items: &[Removal]) -> Result<(), String>;
}
pub struct Session {
    request: Request,
    view: View,
    expires: Instant,
}
impl Session {
    pub fn prepare(request: Request, transport: &mut impl Transport) -> Result<Self, String> {
        validate_id(request.source, &request.id)?;
        let items = canonical(transport.simulate(&request)?)?;
        let mut random = [0u8; 16];
        getrandom::fill(&mut random).map_err(|_| "Aleatoriedade indisponível")?;
        let id = random.iter().map(|b| format!("{b:02x}")).collect();
        Ok(Self {
            view: View {
                id,
                items,
                expires_in_seconds: 120,
                steam_handoff: request.source == Source::Steam,
            },
            request,
            expires: Instant::now() + Duration::from_secs(120),
        })
    }
    pub fn view(&self) -> View {
        let mut view = self.view.clone();
        view.expires_in_seconds = self
            .expires
            .saturating_duration_since(Instant::now())
            .as_secs();
        view
    }
    /// Consumo por valor, inclusive em erro: não há retry com o mesmo plano.
    pub fn execute(self, id: &str, transport: &mut impl Transport) -> Result<(), String> {
        if id != self.view.id || Instant::now() >= self.expires {
            return Err("Plano inválido ou expirado; prepare novamente".into());
        }
        let current = canonical(transport.simulate(&self.request)?)?;
        if current != self.view.items || Instant::now() >= self.expires {
            return Err("A simulação mudou ou expirou; nada foi executado".into());
        }
        transport.execute(&self.request, &current)
    }
}
pub(crate) fn canonical(mut items: Vec<Removal>) -> Result<Vec<Removal>, String> {
    if items.is_empty() || items.len() > 10000 {
        return Err("Lista de remoção vazia ou acima do limite".into());
    }
    for item in &items {
        if item.id.is_empty()
            || item.id.len() > 1024
            || item.id.chars().any(char::is_control)
            || item.version.len() > 512
            || item.version.chars().any(char::is_control)
        {
            return Err("Identidade de remoção ambígua".into());
        }
    }
    for item in &items {
        if fixed_block(item.id.split(';').next().unwrap_or(&item.id)) {
            return Err(format!(
                "Plano inteiro recusado: {} é essencial, kernel, sessão ou o próprio Inodune",
                item.id.split(';').next().unwrap_or(&item.id)
            ));
        }
    }
    items.sort();
    if items.windows(2).any(|w| w[0].id == w[1].id) {
        return Err("Simulação com IDs duplicados".into());
    }
    Ok(items)
}
pub fn validate_id(source: Source, id: &str) -> Result<(), String> {
    let ascii = !id.is_empty()
        && id.len() <= 255
        && id.as_bytes()[0].is_ascii_alphanumeric()
        && !id.contains("..");
    let valid = ascii
        && match source {
            Source::Apt => id
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"+.-:".contains(&c)),
            Source::Rpm => id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"+._-".contains(&c)),
            Source::FlatpakUser | Source::FlatpakSystem => {
                id.split('.').count() >= 3
                    && id.split('.').all(|part| {
                        !part.is_empty()
                            && part
                                .bytes()
                                .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
                    })
            }
            Source::Snap => {
                let (name, rev) = id.split_once('@').map_or((id, None), |(n, r)| (n, Some(r)));
                !name.is_empty()
                    && name
                        .bytes()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"-_".contains(&c))
                    && rev.is_none_or(|r| !r.is_empty() && r.bytes().all(|c| c.is_ascii_digit()))
            }
            Source::Steam => {
                id.len() <= 10
                    && id.bytes().all(|c| c.is_ascii_digit())
                    && id.parse::<u32>().is_ok_and(|v| v > 0)
            }
        };
    if valid {
        Ok(())
    } else {
        Err("ID de pacote inválido para a fonte".into())
    }
}
pub fn package_blocked(name: &str) -> bool {
    fixed_block(name)
}
fn fixed_block(name: &str) -> bool {
    let n = name.split(':').next().unwrap_or(name).to_ascii_lowercase();
    if [
        "bash",
        "coreutils",
        "rust-coreutils",
        "coreutils-from-uutils",
        "packagekit",
        "systemd",
        "apt",
        "dpkg",
        "rpm",
        "dnf",
        "sudo",
        "polkit",
        "polkitd",
        "policykit-1",
        "inodune",
    ]
    .contains(&n.as_str())
        || n.starts_with("kernel-core-")
        || n.starts_with("linux-image")
        || n.starts_with("linux-modules")
        || n.starts_with("gnome-shell")
        || n.starts_with("mutter")
        || n.starts_with("plasma")
        || n.starts_with("kwin")
        || n.starts_with("sddm")
        || n == "gdm"
        || n.starts_with("gdm3")
    {
        return true;
    }
    let release = std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if !release.is_empty()
        && [
            format!("linux-image-{release}"),
            format!("linux-modules-{release}"),
            format!("kernel-core-{release}"),
            format!("kernel-{release}"),
        ]
        .into_iter()
        .any(|package| package == n)
    {
        return true;
    }
    let session = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_ascii_lowercase();
    (session.contains("sway") && n == "sway") || (session.contains("hyprland") && n == "hyprland")
}
pub struct SystemTransport;
impl Transport for SystemTransport {
    fn simulate(&mut self, request: &Request) -> Result<Vec<Removal>, String> {
        if request.ids.len() > 1 {
            return simulate_group(self, request);
        }
        refuse_root()?;
        validate_id(request.source, &request.id)?;
        if fixed_block(&request.id) {
            return Err(format!("Pacote bloqueado: {}", request.id));
        }
        match request.source {
            Source::Apt | Source::Rpm => {
                let resolved = packagekit::resolve(&request.id)?;
                if resolved.len() != 1 {
                    return Err("Resolve ambíguo; escolha uma identidade instalada exata".into());
                }
                let name = package_name(&resolved[0].id)?;
                if name != request.id.split(':').next().unwrap_or(&request.id) {
                    return Err("Resolve não corresponde ao pacote solicitado".into());
                }
                if let Some((_, arch)) = request.id.split_once(':') {
                    if resolved[0].id.split(';').nth(2) != Some(arch) {
                        return Err("Arquitetura divergente no Resolve".into());
                    }
                }
                let items = packagekit::remove(&resolved, true)?;
                if !items.iter().any(|item| item.id == resolved[0].id) {
                    return Err("Simulação não inclui o pacote solicitado; plano bloqueado".into());
                }
                protect_native(request.source, &items)?;
                Ok(items)
            }
            Source::Steam => Ok(vec![Removal {
                id: request.id.clone(),
                version: "Confirmação delegada ao Steam".into(),
            }]),
            Source::FlatpakUser | Source::FlatpakSystem => {
                crate::uninstall_flatpak::simulate(request)
            }
            Source::Snap => crate::uninstall_snap::simulate(request),
        }
    }

    fn execute(&mut self, request: &Request, items: &[Removal]) -> Result<(), String> {
        if request.ids.len() > 1 && !matches!(request.source, Source::Apt | Source::Rpm) {
            for id in &request.ids {
                let one = Request {
                    source: request.source,
                    id: id.clone(),
                    ids: vec![],
                };
                self.execute(&one, items)?;
            }
            return Ok(());
        }
        refuse_root()?;
        match request.source {
            Source::Apt | Source::Rpm => {
                protect_native(request.source, items)?;
                let _ = packagekit::remove(items, false)?;
                Ok(())
            }
            Source::Steam => {
                validate_id(Source::Steam, &request.id)?;
                let status = std::process::Command::new("/usr/bin/xdg-open")
                    .arg(format!("steam://uninstall/{}", request.id))
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .map_err(|_| "Não foi possível abrir o Steam")?;
                if status.success() {
                    Ok(())
                } else {
                    Err("O Steam não confirmou abertura".into())
                }
            }
            Source::FlatpakUser | Source::FlatpakSystem => {
                crate::uninstall_flatpak::execute(request, items)
            }
            Source::Snap => crate::uninstall_snap::execute(request),
        }
    }
}
fn simulate_group(
    transport: &mut SystemTransport,
    request: &Request,
) -> Result<Vec<Removal>, String> {
    refuse_root()?;
    if request.ids.len() > 100 {
        return Err("Seleção acima de 100 pacotes".into());
    }
    let mut seen = BTreeSet::new();
    for id in &request.ids {
        validate_id(request.source, id)?;
        if !seen.insert(id.clone()) {
            return Err("ID repetido na seleção".into());
        }
        if fixed_block(id) {
            return Err(format!("Pacote bloqueado: {id}"));
        }
    }
    if matches!(request.source, Source::Apt | Source::Rpm) {
        let mut resolved = Vec::new();
        for id in &request.ids {
            let found = packagekit::resolve(id)?;
            if found.len() != 1 {
                return Err("Resolve ambíguo; escolha uma identidade instalada exata".into());
            }
            resolved.push(found.into_iter().next().unwrap());
        }
        let items = packagekit::remove(&resolved, true)?;
        protect_native(request.source, &items)?;
        return Ok(items);
    }
    let mut items = Vec::new();
    for id in &request.ids {
        let one = Request {
            source: request.source,
            id: id.clone(),
            ids: vec![],
        };
        items.extend(transport.simulate(&one)?);
    }
    Ok(items)
}
pub fn refuse_root() -> Result<(), String> {
    if rustix::process::geteuid().is_root() || rustix::process::getuid().is_root() {
        Err("O helper recusa execução como root".into())
    } else {
        Ok(())
    }
}
fn package_name(id: &str) -> Result<&str, String> {
    let parts: Vec<_> = id.split(';').collect();
    if parts.len() != 4
        || parts
            .iter()
            .any(|p| p.is_empty() || p.chars().any(char::is_control))
    {
        return Err("PackageKit retornou identidade inválida".into());
    }
    Ok(parts[0])
}
fn protect_native(source: Source, items: &[Removal]) -> Result<(), String> {
    let protected = if source == Source::Apt {
        // O inventário visual é filtrado por apt-mark manual. A proteção de
        // remoção deve incluir TODOS os instalados, inclusive dependências.
        let text = crate::command::run(crate::command::Query::Dpkg)?;
        let all = text
            .lines()
            .filter_map(|line| line.split('\t').next())
            .map(str::to_owned)
            .collect();
        crate::packages::parse_dpkg(&text, &all)?
            .0
            .into_iter()
            .filter(|i| i.protected)
            .map(|i| i.id.split(':').next().unwrap_or(&i.id).to_owned())
            .collect::<BTreeSet<_>>()
    } else {
        // A consulta existente lê protected.d de forma limitada. Qualquer erro
        // de leitura impede estabelecer a política e deve recusar a transação.
        protected_dnf()?
    };
    let core_owners = core_utility_owners(source)?;
    for item in items {
        let name = package_name(&item.id)?;
        validate_id(source, name)?;
        if fixed_block(name) || protected.contains(name) || core_owners.contains(name) {
            return Err(format!("Plano inteiro recusado: pacote protegido {name}"));
        }
    }
    Ok(())
}
pub(crate) fn core_utility_owners(source: Source) -> Result<BTreeSet<String>, String> {
    let apt = source == Source::Apt;
    let text = crate::command::run(crate::command::Query::CoreOwners(apt))?;
    parse_core_utility_owners(source, &text)
}
fn parse_core_utility_owners(source: Source, text: &str) -> Result<BTreeSet<String>, String> {
    let apt = source == Source::Apt;
    let mut names = BTreeSet::new();
    for line in text.lines() {
        let owner = if apt {
            line.split_once(": ")
                .ok_or("Resposta de proprietário inválida")?
                .0
        } else {
            line
        };
        for name in owner.split(", ") {
            let name = name.split(':').next().unwrap_or(name);
            validate_id(source, name)?;
            names.insert(name.to_owned());
        }
    }
    if names.is_empty() {
        return Err("Proprietários dos binários básicos indisponíveis".into());
    }
    Ok(names)
}

fn protected_dnf() -> Result<BTreeSet<String>, String> {
    use std::path::Path;
    let mut names = BTreeSet::new();
    for dir in ["/etc/dnf/protected.d", "/etc/yum/protected.d"] {
        match std::fs::symlink_metadata(Path::new(dir)) {
            Ok(m) if m.is_dir() => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            _ => return Err("protected.d inseguro ou inacessível".into()),
        }
        let entries = std::fs::read_dir(dir).map_err(|_| "protected.d inacessível")?;
        for entry in entries {
            let entry = entry.map_err(|_| "protected.d incompleto")?;
            if entry.path().extension().is_none_or(|e| e != "conf") {
                continue;
            }
            let text = crate::files::read(&entry.path())?;
            for line in text
                .lines()
                .map(str::trim)
                .filter(|s| !s.is_empty() && !s.starts_with('#'))
            {
                // Globs não são convertidos em regex permissiva: recusamos
                // até existir uma correspondência verificável no adaptador.
                validate_id(Source::Rpm, line)
                    .map_err(|_| "Regra protected.d não suportada; plano bloqueado")?;
                names.insert(line.to_owned());
            }
        }
    }
    Ok(names)
}
mod packagekit {
    use super::*;
    use gio::prelude::*;
    use glib::variant::ToVariant;
    use std::sync::{Arc, Mutex};
    const NAME: &str = "org.freedesktop.PackageKit";
    const IFACE: &str = "org.freedesktop.PackageKit.Transaction";
    fn proxy(path: &str, interface: &str) -> Result<gio::DBusProxy, String> {
        gio::DBusProxy::for_bus_sync(
            gio::BusType::System,
            gio::DBusProxyFlags::DO_NOT_LOAD_PROPERTIES,
            None,
            NAME,
            path,
            interface,
            gio::Cancellable::NONE,
        )
        .map_err(|_| "PackageKit indisponível. Verifique se o serviço PackageKit e o barramento do sistema estão instalados e em execução; nenhum pacote foi removido.".to_owned())
    }
    #[derive(Default)]
    struct Reply {
        items: Vec<Removal>,
        done: bool,
        error: Option<String>,
    }

    // Low 16 bits are PkInfoEnum; high bits carry update severity.
    // Discovery remains strict; real progress cannot expand the approved plan.
    fn package_signal(method: &str, info: u32, id: String) -> Result<Removal, String> {
        let expected = if method == "Resolve" { 1 } else { 13 };
        if info & 0xffff != expected {
            return Err(
                "PackageKit propôs ação diferente de remoção/instalado; plano inteiro recusado"
                    .into(),
            );
        }
        package_name(&id)?;
        let fields: Vec<_> = id.split(';').collect();
        Ok(Removal {
            id: id.clone(),
            version: fields[1].into(),
        })
    }
    fn transaction_package_signal(
        method: &str,
        simulate: bool,
        approved: &[Removal],
        info: u32,
        id: String,
    ) -> Result<Option<Removal>, String> {
        if method != "RemovePackages" || simulate {
            return package_signal(method, info, id).map(Some);
        }
        package_name(&id)?;
        if !approved.iter().any(|item| item.id == id) {
            return Err(
                "PackageKit sinalizou pacote fora do plano aprovado; resultado desconhecido".into(),
            );
        }
        // REMOVING, CLEANUP, FINISHED, PREPARING only. These are observations,
        // not additional removals or evidence of transaction success.
        match info & 0xffff {
            13 | 14 | 18 | 21 => Ok(None),
            _ => Err(
                "PackageKit propôs ação diferente da remoção aprovada; resultado desconhecido"
                    .into(),
            ),
        }
    }
    fn removal_parameters(items: &[Removal], simulate: bool) -> glib::Variant {
        let ids = items.iter().map(|i| i.id.clone()).collect::<Vec<_>>();
        (if simulate { 1u64 << 2 } else { 0u64 }, ids, false, false).to_variant()
    }

    fn transaction(
        method: &str,
        args: glib::Variant,
        simulate: bool,
        approved: &[Removal],
    ) -> Result<Vec<Removal>, String> {
        let context = glib::MainContext::new();
        context
            .with_thread_default(|| {
                let manager = proxy("/org/freedesktop/PackageKit", NAME)?;
                let path = manager
                    .call_sync(
                        "CreateTransaction",
                        None,
                        gio::DBusCallFlags::NONE,
                        10000,
                        gio::Cancellable::NONE,
                    )
                    .map_err(|_| "PackageKit indisponível. Verifique o serviço e o barramento do sistema; nenhum pacote foi removido.".to_owned())?;
                let path = path
                    .child_value(0)
                    .str()
                    .ok_or("CreateTransaction inválido")?
                    .to_owned();
                let tx = proxy(&path, IFACE)?;
                let reply = Arc::new(Mutex::new(Reply::default()));
                let state = reply.clone();
                let method_owned = method.to_owned();
                let approved = approved.to_vec();
                let signal = tx.connect_g_signal(move |_, _, name, args| {
                    let Ok(mut reply) = state.lock() else {
                        return;
                    };
                    match name {
                        "Package" => {
                            if let Some((info, id, _summary)) = args.get::<(u32, String, String)>()
                            {
                                if reply.items.len() < 10000 {
                                    match transaction_package_signal(&method_owned, simulate, &approved, info, id) {
                                        Ok(Some(item)) => reply.items.push(item),
                                        Ok(None) => {},
                                        Err(error) => reply.error = Some(error),
                                    }
                                } else {
                                    reply.error = Some("Simulação acima do limite".into());
                                }
                            } else {
                                reply.error = Some(
                                    "PackageKit retornou uma lista incompleta ou ambígua".into(),
                                );
                            }
                        }
                        "Packages" => {
                            if let Some((packages,)) = args.get::<(Vec<(u32, String, String)>,)>() {
                                for (info, id, _) in packages {
                                    if reply.items.len() >= 10000 {
                                        reply.error = Some("Simulação acima do limite".into());
                                        break;
                                    }
                                    match transaction_package_signal(&method_owned, simulate, &approved, info, id) {
                                        Ok(Some(item)) => reply.items.push(item),
                                        Ok(None) => {},
                                        Err(error) => reply.error = Some(error),
                                    }
                                }
                            } else {
                                reply.error = Some("PackageKit retornou lista inválida".into());
                            }
                        }
                        "ErrorCode" => reply.error = Some(
                            "PackageKit recusou a transação; nada será repetido automaticamente"
                                .into(),
                        ),
                        "Finished" => {
                            reply.done = true;
                            if args.get::<(u32, u32)>().is_none_or(|(exit, _)| exit != 1) {
                                reply.error = Some("PackageKit não concluiu com sucesso".into());
                            }
                        }
                        _ => {}
                    }
                });
                // A flag autoriza interação somente na operação real. Na simulação,
                // não permitimos que o D-Bus solicite autorização interativa.
                let flags = if simulate {
                    gio::DBusCallFlags::NONE
                } else {
                    gio::DBusCallFlags::ALLOW_INTERACTIVE_AUTHORIZATION
                };
                tx.call_sync(method, Some(&args), flags, 10000, gio::Cancellable::NONE)
                    .map_err(|e| format!("PackageKit: {e}"))?;
                let start = Instant::now();
                loop {
                    while context.pending() {
                        context.iteration(false);
                    }
                    let state = reply.lock().map_err(|_| "Estado PackageKit indisponível")?;
                    if let Some(error) = &state.error {
                        return Err(error.clone());
                    }
                    if state.done {
                        break;
                    }
                    drop(state);
                    if start.elapsed() > Duration::from_secs(90) {
                        return Err(
                            "PackageKit excedeu 90 s; resultado desconhecido, sem retry".into()
                        );
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
                tx.disconnect(signal);
                let mut state = reply.lock().map_err(|_| "Estado PackageKit indisponível")?;
                Ok(std::mem::take(&mut state.items))
            })
            .map_err(|e| e.to_string())?
    }
    pub fn resolve(id: &str) -> Result<Vec<Removal>, String> {
        transaction(
            "Resolve",
            (1u64 << 2, vec![id.to_owned()]).to_variant(),
            true,
            &[],
        )
    }
    pub fn remove(items: &[Removal], simulate: bool) -> Result<Vec<Removal>, String> {
        transaction(
            "RemovePackages",
            removal_parameters(items, simulate),
            simulate,
            items,
        )
    }
    #[cfg(test)]
    mod signal_tests {
        use super::*;
        #[test]
        fn rejects_non_removal_signals_and_non_installed_resolve_results() {
            for method in ["Resolve", "RemovePackages"] {
                let accepted = if method == "Resolve" { 1 } else { 13 };
                for info in 0..27 {
                    let result =
                        package_signal(method, info, "vendor-tool;1;amd64;installed".into());
                    assert_eq!(result.is_ok(), info == accepted, "{method} {info}");
                }
                for data in [
                    "installed",
                    "installed:stable",
                    "@System",
                    "@System:fedora",
                    "manual:ubuntu-resolute-universe",
                    "auto:ubuntu-resolute-main",
                ] {
                    assert!(package_signal(
                        method,
                        accepted,
                        format!("vendor-tool;1;amd64;{data}")
                    )
                    .is_ok());
                }
                for data in [
                    "available",
                    "manual:ubuntu-resolute-main",
                    "auto:debian",
                    "installed",
                    "@System",
                ] {
                    assert!(
                        package_signal("Resolve", 2, format!("vendor-tool;1;amd64;{data}"))
                            .is_err()
                    );
                }
            }
        }
        #[test]
        fn severity_bits_never_change_discovery_or_simulation_actions() {
            for severity in [0, 1, 8, 0xffff] {
                for info in 0..32 {
                    let id = "hello;1;amd64;manual:ubuntu-main".to_owned();
                    assert_eq!(
                        package_signal("Resolve", info | severity << 16, id.clone()).is_ok(),
                        info == 1
                    );
                    assert_eq!(
                        transaction_package_signal(
                            "RemovePackages",
                            true,
                            &[],
                            info | severity << 16,
                            id
                        )
                        .is_ok(),
                        info == 13
                    );
                }
            }
        }
        #[test]
        fn real_progress_is_allowlisted_and_bound_to_exact_approved_ids() {
            let id = "hello;1;amd64;manual:ubuntu-main".to_owned();
            let approved = [Removal {
                id: id.clone(),
                version: "1".into(),
            }];
            for severity in [0, 8, 0xffff] {
                for info in 0..32 {
                    let result = transaction_package_signal(
                        "RemovePackages",
                        false,
                        &approved,
                        info | severity << 16,
                        id.clone(),
                    );
                    assert_eq!(result.is_ok(), matches!(info, 13 | 14 | 18 | 21), "{info}");
                    if let Ok(item) = result {
                        assert!(item.is_none());
                    }
                }
            }
            for foreign in [
                "other;1;amd64;manual:ubuntu-main",
                "hello;2;amd64;manual:ubuntu-main",
                "hello;1;arm64;manual:ubuntu-main",
                "hello;1;amd64;auto:ubuntu-main",
            ] {
                for info in [13, 14, 18, 21] {
                    assert!(transaction_package_signal(
                        "RemovePackages",
                        false,
                        &approved,
                        info,
                        foreign.into()
                    )
                    .is_err());
                }
            }
        }
        #[test]
        fn real_removal_uses_exact_resimulated_versioned_ids_without_dependencies_or_autoremove() {
            let items = vec![
                Removal {
                    id: "vendor-tool;6.2.0;amd64;installed".into(),
                    version: "6.2.0".into(),
                },
                Removal {
                    id: "vendor-plugin;1.2.3;x86_64;@System".into(),
                    version: "1.2.3".into(),
                },
            ];
            let args = removal_parameters(&items, false)
                .get::<(u64, Vec<String>, bool, bool)>()
                .unwrap();
            assert_eq!(
                args,
                (
                    0,
                    vec![
                        "vendor-tool;6.2.0;amd64;installed".into(),
                        "vendor-plugin;1.2.3;x86_64;@System".into()
                    ],
                    false,
                    false
                )
            );
            assert_eq!(
                removal_parameters(&items, true)
                    .get::<(u64, Vec<String>, bool, bool)>()
                    .unwrap()
                    .0,
                4
            );
        }
    }
}

/// Protocolo limitado, sem comandos de shell nem caminhos fornecidos por IPC.
pub fn read_message(
    input: &mut (impl std::io::Read + std::os::fd::AsFd),
    timeout: Duration,
) -> Result<String, String> {
    use rustix::fs::{fcntl_setfl, OFlags};
    fcntl_setfl(&*input, OFlags::NONBLOCK).map_err(|e| e.to_string())?;
    let start = Instant::now();
    let mut bytes = vec![];
    let mut buf = [0u8; 1];
    loop {
        if start.elapsed() > timeout {
            return Err("Helper expirou; resultado desconhecido, sem retry automático".into());
        }
        match input.read(&mut buf) {
            Ok(0) => return Err("Helper encerrou o canal".into()),
            Ok(_) => {
                if buf[0] == b'\n' {
                    break;
                }
                bytes.push(buf[0]);
                if bytes.len() > 2 * 1024 * 1024 {
                    return Err("Resposta do helper acima do limite".into());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(5))
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    String::from_utf8(bytes).map_err(|_| "Mensagem do helper não é UTF-8".into())
}

/// Antes do envio, o plano é cancelável. Após qualquer tentativa de envio,
/// nem timeout nem Drop podem matar o processo que pode estar modificando
/// pacotes. Fechamos os canais e um reaper aguarda somente nosso próprio filho.
pub struct HelperChild {
    child: Option<std::process::Child>,
    sent: bool,
}
impl HelperChild {
    pub fn new(child: std::process::Child) -> Self {
        Self {
            child: Some(child),
            sent: false,
        }
    }
    pub fn mark_execution_sent(&mut self) {
        self.sent = true;
    }
}
impl std::ops::Deref for HelperChild {
    type Target = std::process::Child;
    fn deref(&self) -> &Self::Target {
        self.child.as_ref().expect("filho retido até Drop")
    }
}
impl std::ops::DerefMut for HelperChild {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.child.as_mut().expect("filho retido até Drop")
    }
}
impl Drop for HelperChild {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            child.stdin.take();
            child.stdout.take();
            child.stderr.take();
            if self.sent {
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
            } else {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}
/// Prazo de notificação, não de cancelamento. O helper continua na chamada
/// original após anunciar resultado desconhecido; ninguém mata a mutação.
pub struct ExecutionDeadline(std::sync::Arc<std::sync::atomic::AtomicBool>);
impl ExecutionDeadline {
    pub fn start(timeout: Duration, notify: impl FnOnce() + Send + 'static) -> Self {
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        };
        let done = Arc::new(AtomicBool::new(false));
        let finished = done.clone();
        std::thread::spawn(move || {
            std::thread::sleep(timeout);
            if !finished.load(Ordering::SeqCst) {
                notify();
            }
        });
        Self(done)
    }
}
impl Drop for ExecutionDeadline {
    fn drop(&mut self) {
        self.0.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Mock {
        calls: usize,
        executed: bool,
        changed: bool,
        blocked: bool,
    }
    impl Transport for Mock {
        fn simulate(&mut self, _: &Request) -> Result<Vec<Removal>, String> {
            self.calls += 1;
            if self.blocked {
                return Err("simulação inclui systemd".into());
            }
            Ok(vec![Removal {
                id: "vendor-tool;1;amd64;installed".into(),
                version: if self.changed && self.calls > 1 {
                    "2"
                } else {
                    "1"
                }
                .into(),
            }])
        }
        fn execute(&mut self, _: &Request, _: &[Removal]) -> Result<(), String> {
            self.executed = true;
            Ok(())
        }
    }
    fn request() -> Request {
        Request {
            source: Source::Apt,
            id: "vendor-tool".into(),
            ids: vec![],
        }
    }

    #[test]
    fn lifecycle_fixture() {
        if let Some(path) = std::env::var_os("INODUNE_HELPER_LIFECYCLE_FIXTURE") {
            std::thread::sleep(Duration::from_millis(150));
            std::fs::write(path, b"completed").unwrap();
        }
    }
    #[test]
    fn dropping_a_sent_helper_reaps_without_killing_the_mutation_fixture() {
        use std::process::{Command, Stdio};
        for sent in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let finished = dir.path().join("finished");
            let child = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "uninstall::tests::lifecycle_fixture",
                    "--nocapture",
                ])
                .env_clear()
                .env("INODUNE_HELPER_LIFECYCLE_FIXTURE", &finished)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            let mut guard = HelperChild::new(child);
            if sent {
                guard.mark_execution_sent();
            }
            drop(guard);
            let start = Instant::now();
            while start.elapsed() < Duration::from_secs(2) && !finished.exists() {
                std::thread::sleep(Duration::from_millis(10));
                if !sent && start.elapsed() > Duration::from_millis(200) {
                    break;
                }
            }
            assert_eq!(finished.exists(), sent);
        }
    }
    #[test]
    fn timeout_notice_never_cancels_the_in_progress_operation() {
        let (tx, rx) = std::sync::mpsc::channel();
        let notice = ExecutionDeadline::start(Duration::from_millis(10), move || {
            tx.send("unknown").unwrap();
        });
        assert_eq!(rx.recv_timeout(Duration::from_secs(1)).unwrap(), "unknown");
        let (release, go) = std::sync::mpsc::channel();
        let operation = std::thread::spawn(move || {
            go.recv().unwrap();
            "completed"
        });
        assert!(!operation.is_finished());
        release.send(()).unwrap();
        assert_eq!(operation.join().unwrap(), "completed");
        drop(notice);
        for name in [
            "bash",
            "coreutils",
            "rust-coreutils",
            "coreutils-from-uutils",
            "packagekit",
            "sddm",
            "kwin-wayland",
            "mutter",
            "gnome-shell",
            "apt",
            "dpkg",
            "inodune",
        ] {
            assert!(fixed_block(name), "{name}");
        }
        for name in ["aptitude", "linux-firmware", "network-manager", "hello"] {
            assert!(!fixed_block(name), "{name}");
        }
    }
    #[test]
    fn validates_source_specific_ids_without_shell_metacharacters() {
        for source in [
            Source::Apt,
            Source::Rpm,
            Source::FlatpakUser,
            Source::Snap,
            Source::Steam,
        ] {
            for id in ["-x", "x y", "x;y", "$(id)", "../escape", "x\n"] {
                assert!(validate_id(source, id).is_err());
            }
        }
        assert!(validate_id(Source::Steam, "12345").is_ok());
        assert!(validate_id(Source::Steam, "123/4").is_err());
        assert!(validate_id(Source::FlatpakUser, "org.example.App").is_ok());
    }
    #[test]
    fn resimulates_exact_list_before_single_use_execution() {
        let mut mock = Mock::default();
        let plan = Session::prepare(request(), &mut mock).unwrap();
        let id = plan.view().id;
        plan.execute(&id, &mut mock).unwrap();
        assert!(mock.executed);
        assert_eq!(mock.calls, 2);
    }
    #[test]
    fn changed_expired_forged_and_blocked_plans_never_execute() {
        for mode in 0..4 {
            let mut mock = Mock::default();
            if mode == 3 {
                mock.blocked = true;
                assert!(Session::prepare(request(), &mut mock).is_err());
                continue;
            }
            let mut plan = Session::prepare(request(), &mut mock).unwrap();
            let mut id = plan.view().id;
            if mode == 0 {
                mock.changed = true;
            }
            if mode == 1 {
                plan.expires = Instant::now() - Duration::from_secs(1);
            }
            if mode == 2 {
                id = "forged".into();
            }
            assert!(plan.execute(&id, &mut mock).is_err());
            assert!(!mock.executed);
        }
    }
    #[test]
    fn core_binary_owner_queries_protect_nonstandard_providers() {
        let owners = parse_core_utility_owners(
            Source::Apt,
            "custom-tools:amd64: /usr/bin/ls\nother-tools: /usr/bin/cat\n",
        )
        .unwrap();
        assert!(owners.contains("custom-tools"));
        assert!(owners.contains("other-tools"));
        assert!(parse_core_utility_owners(Source::Apt, "hello;id: /usr/bin/ls").is_err());
        assert!(parse_core_utility_owners(Source::Rpm, "").is_err());
        assert!(parse_core_utility_owners(Source::Rpm, "custom-tools\n")
            .unwrap()
            .contains("custom-tools"));
    }
    #[test]
    fn fixed_session_and_kernel_packages_are_always_blocked() {
        for n in [
            "bash",
            "coreutils",
            "rust-coreutils",
            "coreutils-from-uutils",
            "packagekit",
            "kernel-core-6.1",
            "linux-image-6.1",
            "linux-modules-extra",
            "plasma-desktop",
            "gdm",
            "gnome-shell",
            "sudo",
            "polkit",
            "inodune",
        ] {
            assert!(fixed_block(n), "{n}");
        }
        for n in ["systemd-libs", "NetworkManager", "kernel-core"] {
            assert!(!fixed_block(n), "{n}");
        }
    }
}
