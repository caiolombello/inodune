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
    pub id: String,
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
fn canonical(mut items: Vec<Removal>) -> Result<Vec<Removal>, String> {
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
            return Err("Plano inteiro recusado: pacote essencial, kernel ou sessão".into());
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
fn fixed_block(name: &str) -> bool {
    let n = name.split(':').next().unwrap_or(name).to_ascii_lowercase();
    [
        "kernel",
        "linux-image",
        "linux-modules",
        "linux-headers",
        "plasma",
        "systemd",
        "polkit",
        "gdm",
        "gnome-shell",
        "network-manager",
        "networkmanager",
    ]
    .iter()
    .any(|p| n.starts_with(p))
        || [
            "sudo",
            "apt",
            "dpkg",
            "rpm",
            "dnf",
            "packagekit",
            "glibc",
            "libc6",
        ]
        .contains(&n.as_str())
}
pub struct SystemTransport;
impl Transport for SystemTransport {
    fn simulate(&mut self, request: &Request) -> Result<Vec<Removal>, String> {
        refuse_root()?;
        validate_id(request.source, &request.id)?;
        if fixed_block(&request.id) {
            return Err("Pacote essencial, kernel ou sessão protegido".into());
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
    for item in items {
        let name = package_name(&item.id)?;
        validate_id(source, name)?;
        if fixed_block(name) || protected.contains(name) {
            return Err(format!("Plano inteiro recusado: pacote protegido {name}"));
        }
    }
    Ok(())
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
        .map_err(|e| format!("PackageKit indisponível: {e}"))
    }
    #[derive(Default)]
    struct Reply {
        items: Vec<Removal>,
        done: bool,
        error: Option<String>,
    }
    fn transaction(
        method: &str,
        args: glib::Variant,
        simulate: bool,
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
                    .map_err(|e| e.to_string())?;
                let path = path
                    .child_value(0)
                    .str()
                    .ok_or("CreateTransaction inválido")?
                    .to_owned();
                let tx = proxy(&path, IFACE)?;
                let reply = Arc::new(Mutex::new(Reply::default()));
                let state = reply.clone();
                let signal = tx.connect_g_signal(move |_, _, name, args| {
                    let Ok(mut reply) = state.lock() else {
                        return;
                    };
                    match name {
                        "Package" => {
                            if let Some((_info, id, _summary)) = args.get::<(u32, String, String)>()
                            {
                                if reply.items.len() < 10000 {
                                    let version = id.split(';').nth(1).unwrap_or("").to_owned();
                                    reply.items.push(Removal { id, version });
                                } else {
                                    reply.error = Some("Simulação acima do limite".into());
                                }
                            } else {
                                reply.error = Some("PackageKit retornou uma lista incompleta ou ambígua".into());
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
        )
    }
    pub fn remove(items: &[Removal], simulate: bool) -> Result<Vec<Removal>, String> {
        let ids = items.iter().map(|i| i.id.clone()).collect::<Vec<_>>();
        transaction(
            "RemovePackages",
            (if simulate { 1u64 << 2 } else { 0 }, ids, false, false).to_variant(),
            simulate,
        )
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
    fn fixed_session_and_kernel_packages_are_always_blocked() {
        for n in [
            "kernel-core",
            "linux-image-6.1",
            "linux-modules-extra",
            "plasma-desktop",
            "gdm",
            "gnome-shell",
            "systemd-libs",
            "sudo",
            "polkit",
            "NetworkManager",
        ] {
            assert!(fixed_block(n), "{n}");
        }
    }
}
