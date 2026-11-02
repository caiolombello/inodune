//! Textos legíveis para planos de desinstalação e erros do PackageKit.
//! Somente apresentação: nada aqui decide o que é removido.
use crate::uninstall::{Removal, RemovalDetail};
use crate::{InstalledItem, Source};

/// Bytes em unidades binárias com vírgula decimal (pt-BR): `9 MiB`, `20,2 MiB`.
pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut unit = 0;
    let mut value = bytes as f64;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        return format!("{bytes} B");
    }
    let rounded = (value * 10.0).round() / 10.0;
    let text = if rounded.fract() == 0.0 {
        format!("{rounded:.0}")
    } else {
        format!("{rounded:.1}").replace('.', ",")
    };
    format!("{text} {}", UNITS[unit])
}

fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// Nome, arquitetura e versão exibíveis a partir da identidade técnica.
fn identity(source: Source, item: &Removal) -> (String, Option<String>, String) {
    match source {
        Source::Apt | Source::Rpm => {
            let fields: Vec<_> = item.id.split(';').collect();
            if fields.len() == 4 && !fields[0].is_empty() {
                let arch = (!fields[2].is_empty()).then(|| fields[2].to_owned());
                return (fields[0].to_owned(), arch, item.version.clone());
            }
            (item.id.clone(), None, item.version.clone())
        }
        Source::FlatpakUser | Source::FlatpakSystem => {
            let fields: Vec<_> = item.id.split('/').collect();
            let short: String = item.version.chars().take(12).collect();
            if fields.len() == 4 {
                return (fields[1].to_owned(), Some(fields[2].to_owned()), short);
            }
            (item.id.clone(), None, short)
        }
        Source::Snap | Source::Steam => (item.id.clone(), None, item.version.clone()),
    }
}

fn find_item<'a>(
    source: Source,
    name: &str,
    arch: Option<&str>,
    inventory: &'a [InstalledItem],
) -> Option<&'a InstalledItem> {
    let same = |item: &&InstalledItem| item.source == source;
    let with_arch = arch.map(|arch| format!("{name}:{arch}"));
    inventory
        .iter()
        .filter(same)
        .find(|item| Some(&item.id) == with_arch.as_ref())
        .or_else(|| inventory.iter().filter(same).find(|item| item.id == name))
        .or_else(|| {
            inventory
                .iter()
                .filter(same)
                .find(|item| item.id.split(':').next() == Some(name))
        })
}

/// Detalhes de cada remoção: nome amigável, versão, tamanho instalado e se foi
/// pedido pelo usuário ou entra como dependência.
pub fn describe(
    source: Source,
    items: &[Removal],
    requested: &[String],
    inventory: &[InstalledItem],
) -> Vec<RemovalDetail> {
    items
        .iter()
        .map(|item| {
            let (name, arch, version) = identity(source, item);
            let found = find_item(source, &name, arch.as_deref(), inventory);
            let is_requested = requested.iter().any(|id| {
                let base = id.split(':').next().unwrap_or(id);
                let id_arch = id.split_once(':').map(|(_, a)| a);
                (base == name && id_arch.is_none_or(|a| Some(a) == arch.as_deref()))
                    || *id == item.id
                    || found.is_some_and(|f| f.id == *id)
            });
            RemovalDetail {
                id: item.id.clone(),
                name: found
                    .map(|f| f.name.clone())
                    .filter(|n| !n.is_empty())
                    .unwrap_or(name),
                version: found
                    .and_then(|f| f.version.clone())
                    .filter(|_| matches!(source, Source::FlatpakUser | Source::FlatpakSystem))
                    .unwrap_or(version),
                arch,
                size_bytes: found.and_then(|f| f.size_bytes),
                requested: is_requested || source == Source::Steam,
            }
        })
        .collect()
}

fn detail_line(detail: &RemovalDetail) -> String {
    let size = detail
        .size_bytes
        .map_or_else(|| "tamanho desconhecido".to_owned(), human_bytes);
    format!("  • {} {} · {size}", detail.name, detail.version)
}

/// Resumo usado no diálogo nativo e na CLI antes da confirmação.
pub fn confirmation_text(details: &[RemovalDetail], steam: bool) -> String {
    if steam {
        let names = details
            .iter()
            .map(|d| format!("  • {}", d.name))
            .collect::<Vec<_>>()
            .join("\n");
        return format!(
            "Abrir a confirmação de desinstalação do Steam?\n\n{names}\n\nO Steam faz a remoção; o Inodune não confirma o resultado."
        );
    }
    let chosen: Vec<_> = details.iter().filter(|d| d.requested).collect();
    let deps: Vec<_> = details.iter().filter(|d| !d.requested).collect();
    let known: u64 = details.iter().filter_map(|d| d.size_bytes).sum();
    let unknown = details.iter().filter(|d| d.size_bytes.is_none()).count();
    let mut size = human_bytes(known);
    if unknown > 0 {
        size.push_str(&format!(
            " + {} sem tamanho",
            plural(unknown, "pacote", "pacotes")
        ));
    }
    let mut text = format!(
        "Desinstalar {}{}? Total: {} · {} instalados.\n\n",
        plural(chosen.len(), "programa", "programas"),
        if deps.is_empty() {
            String::new()
        } else {
            format!(" e {}", plural(deps.len(), "dependência", "dependências"))
        },
        plural(details.len(), "pacote", "pacotes"),
        size
    );
    text.push_str("Selecionados:\n");
    for d in &chosen {
        text.push_str(&detail_line(d));
        text.push('\n');
    }
    if !deps.is_empty() {
        text.push_str("\nDependências que também serão removidas:\n");
        for d in &deps {
            text.push_str(&detail_line(d));
            text.push('\n');
        }
    }
    text.push_str("\nSem autoremove nem purge. O sistema pode pedir sua senha (polkit).");
    text
}

fn clean_details(details: &str) -> String {
    let text: String = details
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(300)
        .collect();
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Mensagem específica para o sinal ErrorCode (PkErrorEnum).
pub fn packagekit_error(code: u32, details: &str, simulate: bool) -> String {
    let details = clean_details(details);
    let suffix = if details.is_empty() {
        String::new()
    } else {
        format!(" Detalhe do PackageKit: {details}")
    };
    let nothing = "Nenhum pacote foi removido.";
    let check = "Confira a lista de programas antes de tentar de novo.";
    match code {
        48 => format!("Autorização negada ou cancelada na janela de senha do sistema (polkit). {nothing}"),
        17 | 65 if simulate => format!("A simulação foi cancelada. {nothing}"),
        17 | 65 => format!("A transação foi cancelada antes de terminar. {check}"),
        13 => format!("Outros pacotes dependem desta seleção, e o Inodune não remove dependentes automaticamente. {nothing}{suffix}"),
        20 => format!("Pacote protegido pelo sistema; o PackageKit não permite removê-lo. {nothing}"),
        26 | 67 => format!("Outro gerenciador de pacotes está em uso (apt, dnf ou atualização automática). Aguarde terminar e tente de novo. {nothing}"),
        7 | 8 => format!("O pacote não está mais instalado ou não foi encontrado. Atualize a lista. {nothing}"),
        59 => format!("O gerenciador de pacotes não conseguiu remover o pacote. O resultado pode ser parcial. {check}{suffix}"),
        _ if simulate => format!("O PackageKit recusou a transação (código {code}).{suffix} {nothing}"),
        _ => format!("O PackageKit recusou a transação (código {code}).{suffix} {check} Nada será repetido automaticamente."),
    }
}

/// Mensagem para Finished sem sucesso quando não houve ErrorCode antes.
pub fn packagekit_exit(exit: u32, simulate: bool) -> String {
    let check = "Confira a lista de programas antes de tentar de novo.";
    match exit {
        3 | 9 if simulate => "A simulação foi cancelada. Nenhum pacote foi removido.".into(),
        3 | 9 => format!("A transação foi cancelada antes de terminar. {check}"),
        6 => format!("A transação foi interrompida pelo sistema; resultado desconhecido. {check}"),
        _ => format!("O PackageKit terminou sem sucesso (código de saída {exit}) e não informou o motivo. {check}"),
    }
}

/// Erro na chamada D-Bus (antes de qualquer sinal).
pub fn packagekit_call_error(error: &str) -> String {
    let lower = error.to_ascii_lowercase();
    let denied = ["notauthorized", "refusedbypolicy", "not authorized"];
    if denied.iter().any(|needle| lower.contains(needle)) {
        "Autorização negada ou cancelada na janela de senha do sistema (polkit). Nenhum pacote foi removido.".into()
    } else if lower.contains("serviceunknown") || lower.contains("name has no owner") {
        "PackageKit indisponível. Verifique se o serviço está instalado e em execução; nenhum pacote foi removido.".into()
    } else {
        format!("Falha ao falar com o PackageKit: {}", clean_details(error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(source: Source, id: &str, name: &str, size: Option<u64>) -> InstalledItem {
        let mut item = InstalledItem::new(source, id, name);
        item.size_bytes = size;
        item.version = Some("v".into());
        item
    }

    #[test]
    fn human_bytes_uses_binary_units_and_decimal_comma() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(1023), "1023 B");
        assert_eq!(human_bytes(2_000_000), "1,9 MiB");
        assert_eq!(human_bytes(9 * 1024 * 1024), "9 MiB");
        assert_eq!(human_bytes(14_500_000_000), "13,5 GiB");
    }

    #[test]
    fn describe_marks_requested_and_dependencies_with_friendly_names() {
        let removals = vec![
            Removal {
                id: "aisleriot;1:3.22.35-5;amd64;manual:ubuntu-resolute-universe".into(),
                version: "1:3.22.35-5".into(),
            },
            Removal {
                id: "aisleriot-data;1:3.22.35-5;all;auto:ubuntu-resolute-universe".into(),
                version: "1:3.22.35-5".into(),
            },
        ];
        let inventory = vec![
            item(Source::Apt, "aisleriot", "aisleriot", Some(9 * 1024 * 1024)),
            item(Source::Apt, "aisleriot-data", "aisleriot-data", None),
        ];
        let requested = ["aisleriot".to_owned()];
        let details = describe(Source::Apt, &removals, &requested, &inventory);
        assert_eq!(details[0].name, "aisleriot");
        assert_eq!(details[0].version, "1:3.22.35-5");
        assert!(details[0].requested);
        assert!(!details[1].requested);
        assert_eq!(details[1].size_bytes, None);
        let text = confirmation_text(&details, false);
        let first = text.lines().next().unwrap();
        assert_eq!(
            first,
            "Desinstalar 1 programa e 1 dependência? \
             Total: 2 pacotes · 9 MiB + 1 pacote sem tamanho instalados."
        );
        assert!(text.contains("Dependências que também serão removidas:"));
        assert!(text.contains("  • aisleriot-data 1:3.22.35-5 · tamanho desconhecido"));
        assert!(!text.contains("manual:ubuntu"), "raw id must not be shown");
    }

    #[test]
    fn describe_matches_arch_qualified_ids() {
        let removals = vec![Removal {
            id: "libfoo;2;i386;installed".into(),
            version: "2".into(),
        }];
        let inventory = vec![
            item(Source::Apt, "libfoo:amd64", "libfoo", Some(10)),
            item(Source::Apt, "libfoo:i386", "libfoo", Some(20)),
        ];
        let i386 = ["libfoo:i386".to_owned()];
        let details = describe(Source::Apt, &removals, &i386, &inventory);
        assert_eq!(details[0].size_bytes, Some(20));
        assert!(details[0].requested);
        let amd64 = ["libfoo:amd64".to_owned()];
        let other = describe(Source::Apt, &removals, &amd64, &inventory);
        assert!(!other[0].requested);
    }

    #[test]
    fn packagekit_errors_are_specific() {
        assert!(packagekit_error(48, "", false).contains("polkit"));
        assert!(packagekit_error(13, "x requires y", true).contains("dependem"));
        assert!(packagekit_error(13, "x requires y", true).contains("x requires y"));
        assert!(packagekit_error(20, "", false).contains("protegido"));
        assert!(packagekit_error(26, "", false).contains("Outro gerenciador"));
        assert!(packagekit_error(17, "", false).contains("cancelada"));
        assert!(packagekit_error(999, "a\u{7}b", false).contains("código 999"));
        assert!(!packagekit_error(999, "a\u{7}b", false).contains('\u{7}'));
        assert!(packagekit_exit(3, true).contains("cancelada"));
        assert!(packagekit_exit(2, false).contains("código de saída 2"));
        let refused = "GDBus.Error:org.freedesktop.PackageKit.Transaction.RefusedByPolicy: x";
        assert!(packagekit_call_error(refused).contains("polkit"));
    }
}
