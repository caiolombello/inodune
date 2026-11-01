//! Resolve somente transações de uninstall pela libflatpak, abortando em
//! ready-pre-auth. Nunca usa lista inferida a partir de sufixos de nomes.
use crate::{
    command::{run, Query},
    uninstall::{validate_id, Removal, Request},
    Source,
};
use glib::{
    prelude::*,
    translate::{from_glib_full, from_glib_none},
};
use std::{
    cell::RefCell,
    ffi::{c_char, c_void, CStr, CString},
    rc::Rc,
};
type Ptr = *mut c_void;
type ErrorPtr = *mut *mut glib::ffi::GError;
type New = unsafe extern "C" fn(Ptr, ErrorPtr) -> Ptr;
type NewTx = unsafe extern "C" fn(Ptr, Ptr, ErrorPtr) -> Ptr;
type Set = unsafe extern "C" fn(Ptr, i32);
type Add = unsafe extern "C" fn(Ptr, *const c_char, ErrorPtr) -> i32;
type Run = unsafe extern "C" fn(Ptr, Ptr, ErrorPtr) -> i32;
type Text = unsafe extern "C" fn(Ptr) -> *const c_char;
type GetOps = unsafe extern "C" fn(Ptr) -> *mut glib::ffi::GList;
type GetType = unsafe extern "C" fn(Ptr) -> i32;
type TypeText = unsafe extern "C" fn(i32) -> *const c_char;
type Metadata = unsafe extern "C" fn(Ptr) -> *mut glib::ffi::GKeyFile;
type Installed = unsafe extern "C" fn(
    Ptr,
    i32,
    *const c_char,
    *const c_char,
    *const c_char,
    Ptr,
    ErrorPtr,
) -> Ptr;
struct Api {
    library: libloading::Library,
}
impl Api {
    fn open() -> Result<Self, String> {
        for path in [
            "/usr/lib64/libflatpak.so.0",
            "/usr/lib/x86_64-linux-gnu/libflatpak.so.0",
            "/usr/lib/aarch64-linux-gnu/libflatpak.so.0",
        ] {
            // Caminhos absolutos do sistema, sem PATH/LD_LIBRARY_PATH do renderer.
            if let Ok(library) = unsafe { libloading::Library::new(path) } {
                return Ok(Self { library });
            }
        }
        Err(
            "libflatpak indisponível; não é possível simular refs relacionados com segurança"
                .into(),
        )
    }
    unsafe fn symbol<T: Copy>(&self, name: &[u8]) -> Result<T, String> {
        self.library
            .get::<T>(name)
            .map(|s| *s)
            .map_err(|_| "API libflatpak incompatível; desinstalação bloqueada".into())
    }
}
unsafe fn text(ptr: *const c_char) -> Result<String, String> {
    if ptr.is_null() {
        return Err("Metadado libflatpak ausente".into());
    }
    CStr::from_ptr(ptr)
        .to_str()
        .map(str::to_owned)
        .map_err(|_| "Metadado libflatpak ambíguo".into())
}
unsafe fn object(ptr: Ptr, error: *mut glib::ffi::GError) -> Result<glib::Object, String> {
    if !error.is_null() {
        let error: glib::Error = from_glib_full(error);
        return Err(format!("libflatpak: {error}"));
    }
    if ptr.is_null() {
        return Err("Objeto libflatpak indisponível".into());
    }
    Ok(from_glib_full(ptr.cast::<glib::gobject_ffi::GObject>()))
}
fn validate_ref(reference: &str) -> Result<Vec<&str>, String> {
    let parts: Vec<_> = reference.split('/').collect();
    if parts.len() != 4 || !matches!(parts[0], "app" | "runtime") {
        return Err("Ref Flatpak inválido".into());
    }
    validate_id(Source::FlatpakUser, parts[1])?;
    for part in &parts[2..] {
        if part.is_empty()
            || part.len() > 128
            || !part.as_bytes()[0].is_ascii_alphanumeric()
            || !part
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
            || part.contains("..")
        {
            return Err("Arch/branch Flatpak inválido".into());
        }
    }
    Ok(parts)
}
fn validate_operations(
    root: &str,
    ops: Vec<(String, String, Option<String>)>,
) -> Result<Vec<Removal>, String> {
    let mut items = vec![];
    let mut root_seen = false;
    for (reference, commit, extension_of) in ops {
        let parts = validate_ref(&reference)?;
        if commit.len() != 64 || !commit.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err("Commit Flatpak desconhecido; plano bloqueado".into());
        }
        if reference == root {
            root_seen = true;
        } else if parts[0] != "runtime" || extension_of.as_deref() != Some(root) {
            return Err(
                "A transação inclui outro app ou runtime base; plano inteiro bloqueado".into(),
            );
        }
        items.push(Removal {
            id: reference,
            version: commit,
        });
    }
    if !root_seen {
        return Err("Transação Flatpak sem o aplicativo solicitado".into());
    }
    Ok(items)
}
pub(super) fn simulate(request: &Request) -> Result<Vec<Removal>, String> {
    let listing = run(Query::Flatpak(request.source == Source::FlatpakUser))?;
    let matches: Vec<_> = listing
        .lines()
        .map(|line| line.split('\t').collect::<Vec<_>>())
        .filter(|f| f.first() == Some(&request.id.as_str()))
        .collect();
    if matches.len() != 1 || matches[0].len() != 6 {
        return Err(
            "App Flatpak ausente ou múltiplas branches/arquiteturas; escolha não é inequívoca"
                .into(),
        );
    }
    let fields = &matches[0];
    let root = format!("app/{}/{}/{}", request.id, fields[5], fields[4]);
    validate_ref(&root)?;
    let api = Rc::new(Api::open()?);
    // As assinaturas correspondem à API C libflatpak estável. Todos os objetos
    // transfer-full são convertidos em wrappers GLib; a biblioteca fica viva
    // até o último callback/objeto ser descartado. Nenhuma operação é executada.
    unsafe {
        let new: New = api.symbol(if request.source == Source::FlatpakUser {
            b"flatpak_installation_new_user\0"
        } else {
            b"flatpak_installation_new_system\0"
        })?;
        let mut error = std::ptr::null_mut();
        let installation = object(new(std::ptr::null_mut(), &mut error), error)?;
        let new_tx: NewTx = api.symbol(b"flatpak_transaction_new_for_installation\0")?;
        error = std::ptr::null_mut();
        let tx = object(
            new_tx(
                installation.as_ptr().cast(),
                std::ptr::null_mut(),
                &mut error,
            ),
            error,
        )?;
        for (name, value) in [
            (
                b"flatpak_transaction_set_disable_dependencies\0".as_slice(),
                1,
            ),
            (b"flatpak_transaction_set_disable_related\0".as_slice(), 0),
            (b"flatpak_transaction_set_no_pull\0".as_slice(), 1),
            (b"flatpak_transaction_set_disable_prune\0".as_slice(), 1),
        ] {
            let set: Set = api.symbol(name)?;
            set(tx.as_ptr().cast(), value);
        }
        // Esse sinal existe desde 1.9.1; ausência recusa, sem fallback pós-auth.
        if glib::subclass::SignalId::lookup("ready-pre-auth", tx.type_()).is_none() {
            return Err("libflatpak sem ready-pre-auth; simulação segura indisponível".into());
        }
        let ops: GetOps = api.symbol(b"flatpak_transaction_get_operations\0")?;
        let get_ref: Text = api.symbol(b"flatpak_transaction_operation_get_ref\0")?;
        let get_type: GetType =
            api.symbol(b"flatpak_transaction_operation_get_operation_type\0")?;
        let type_text: TypeText = api.symbol(b"flatpak_transaction_operation_type_to_string\0")?;
        let old_metadata: Metadata =
            api.symbol(b"flatpak_transaction_operation_get_old_metadata\0")?;
        let installed: Installed = api.symbol(b"flatpak_installation_get_installed_ref\0")?;
        let commit: Text = api.symbol(b"flatpak_ref_get_commit\0")?;
        let captured = Rc::new(RefCell::new(None));
        let result = captured.clone();
        let held = api.clone();
        let install = installation.clone();
        let signal = tx.connect_local("ready-pre-auth", false, move |values| {
            let _held = &held;
            let outcome = (|| {
                let tx = values[0]
                    .get::<glib::Object>()
                    .map_err(|_| "Sinal libflatpak inválido")?;
                let list = ops(tx.as_ptr().cast());
                let mut node = list;
                let mut collected = vec![];
                let outcome: Result<Vec<(String, String, Option<String>)>, String> = (|| {
                    while !node.is_null() {
                        if collected.len() >= 10000 {
                            return Err("Transação Flatpak acima do limite".into());
                        }
                        let op = (*node).data;
                        if text(type_text(get_type(op)))? != "uninstall" {
                            return Err("Operação Flatpak inesperada".into());
                        }
                        let reference = text(get_ref(op))?;
                        let parts = validate_ref(&reference)?;
                        let name = CString::new(parts[1]).map_err(|_| "Ref inválido")?;
                        let arch = CString::new(parts[2]).map_err(|_| "Ref inválido")?;
                        let branch = CString::new(parts[3]).map_err(|_| "Ref inválido")?;
                        let mut error = std::ptr::null_mut();
                        let current = object(
                            installed(
                                install.as_ptr().cast(),
                                if parts[0] == "app" { 0 } else { 1 },
                                name.as_ptr(),
                                arch.as_ptr(),
                                branch.as_ptr(),
                                std::ptr::null_mut(),
                                &mut error,
                            ),
                            error,
                        )?;
                        let hash = text(commit(current.as_ptr().cast()))?;
                        let metadata = old_metadata(op);
                        let extension = if metadata.is_null() {
                            None
                        } else {
                            let key: glib::KeyFile = from_glib_none(metadata);
                            key.string("ExtensionOf", "ref").ok().map(|s| s.to_string())
                        };
                        collected.push((reference, hash, extension));
                        node = (*node).next;
                    }
                    Ok(collected)
                })(
                );
                let mut node = list;
                while !node.is_null() {
                    glib::gobject_ffi::g_object_unref((*node).data.cast());
                    node = (*node).next;
                }
                glib::ffi::g_list_free(list);
                outcome
            })();
            *result.borrow_mut() = Some(outcome);
            Some(false.to_value()) // Aborta sempre, antes de autorização/mutação.
        });
        let add: Add = api.symbol(b"flatpak_transaction_add_uninstall\0")?;
        let root_c = CString::new(root.as_str()).map_err(|_| "Ref inválido")?;
        error = std::ptr::null_mut();
        if add(tx.as_ptr().cast(), root_c.as_ptr(), &mut error) == 0 {
            if !error.is_null() {
                let _: glib::Error = from_glib_full(error);
            }
            return Err("Não foi possível preparar a transação Flatpak".into());
        }
        let run: Run = api.symbol(b"flatpak_transaction_run\0")?;
        error = std::ptr::null_mut();
        run(tx.as_ptr().cast(), std::ptr::null_mut(), &mut error);
        if !error.is_null() {
            let _: glib::Error = from_glib_full(error);
        }
        tx.disconnect(signal);
        let result = captured.borrow_mut().take().ok_or(
            "libflatpak não forneceu a lista completa antes de autorizar; remoção bloqueada",
        )??;
        validate_operations(&root, result)
    }
}
pub(super) fn execute(request: &Request, items: &[Removal]) -> Result<(), String> {
    let roots: Vec<_> = items.iter().filter(|i| i.id.starts_with("app/")).collect();
    if roots.len() != 1 {
        return Err("Plano Flatpak ambíguo".into());
    }
    validate_ref(&roots[0].id)?;
    let mut command = std::process::Command::new("/usr/bin/flatpak");
    command
        .args([
            "uninstall",
            if request.source == Source::FlatpakUser {
                "--user"
            } else {
                "--system"
            },
            "--noninteractive",
            "--",
            roots[0].id.as_str(),
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    let status = command.status().map_err(|_| "Flatpak indisponível")?;
    if status.success() {
        Ok(())
    } else {
        Err("Flatpak não confirmou remoção completa; consulte o inventário".into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_exact_app_and_authoritative_related_extensions_but_never_base_runtimes() {
        let root = "app/org.example.Tool/x86_64/stable";
        let hash = "a".repeat(64);
        let app = (root.into(), hash.clone(), None);
        let locale = (
            "runtime/org.example.Tool.Locale/x86_64/stable".into(),
            hash.clone(),
            Some(root.into()),
        );
        assert_eq!(
            validate_operations(root, vec![app.clone(), locale])
                .unwrap()
                .len(),
            2
        );
        assert!(validate_operations(
            root,
            vec![
                app.clone(),
                (
                    "runtime/org.base.Platform/x86_64/24.08".into(),
                    hash.clone(),
                    None
                )
            ]
        )
        .is_err());
        assert!(validate_operations(
            root,
            vec![
                app,
                (
                    "runtime/org.example.Tool.Debug/x86_64/stable".into(),
                    hash,
                    Some("app/org.other.App/x86_64/stable".into())
                )
            ]
        )
        .is_err());
    }
    #[test]
    fn refs_reject_runtime_selection_options_metacharacters_and_ambiguous_paths() {
        for reference in [
            "app/-x/x86_64/stable",
            "app/org.example.App/x86_64/--unused",
            "app/org.example.App/x86_64/../stable",
            "app/org.example.App/x86_64/a;b",
        ] {
            assert!(validate_ref(reference).is_err());
        }
    }
}
