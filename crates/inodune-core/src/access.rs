//! Directory write policy shared by storage, scan and cleanup.
//! Group-write is accepted only for the caller's own private group.
//! Other-write is refused. A failed group lookup is a refusal.
use std::{
    collections::HashMap,
    ffi::CStr,
    sync::{LazyLock, Mutex},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Caller {
    pub uid: u32,
    pub gid: u32,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupCensus {
    /// Supplementary members from `getgrgid` (`gr_mem`). Primary users are separate.
    pub supplementary: Vec<String>,
    /// Users whose primary group id is this group.
    pub primary_users: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupLookupError;

pub trait GroupQuery {
    fn uid_gid(&self) -> Result<(u32, u32), GroupLookupError>;
    fn user_name(&self) -> Result<String, GroupLookupError>;
    fn census(&self, gid: u32) -> Result<GroupCensus, GroupLookupError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteRole {
    /// Ancestors of private storage, including root-owned sticky temporary directories.
    StorageAncestor,
    /// A directory inside an approved cleanup root. The owner must be the caller.
    CleanupComponent,
    /// The approved cleanup root. Mode is checked; a non-writable root may belong to another user.
    CleanupRoot,
    /// An existing private directory such as Trash or the report registry.
    PrivateStore,
    /// A cache file that cleanup would rename.
    CacheLeaf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteRefusal {
    OtherWrite,
    SharedGroupWrite,
    GroupLookupFailed,
    UnexpectedOwner,
}

pub fn describe_write_refusal(refusal: WriteRefusal) -> &'static str {
    match refusal {
        WriteRefusal::OtherWrite => {
            "Pasta gravável por qualquer usuário no sistema; limpeza recusada sem alterar permissões."
        }
        WriteRefusal::SharedGroupWrite => {
            "Pasta gravável por outros membros do grupo; limpeza recusada sem alterar permissões."
        }
        WriteRefusal::GroupLookupFailed => {
            "Grupo da pasta não pôde ser confirmado como exclusivo do usuário; limpeza recusada sem alterar permissões."
        }
        WriteRefusal::UnexpectedOwner => {
            "Pasta pertence a outro usuário; limpeza recusada sem alterar permissões."
        }
    }
}

pub fn stat_write_allowed(
    role: WriteRole,
    owner_uid: u32,
    owner_gid: u32,
    mode: u32,
    groups: &dyn GroupQuery,
) -> Result<(), WriteRefusal> {
    if role == WriteRole::StorageAncestor && owner_uid == 0 && mode & 0o1000 != 0 {
        return Ok(());
    }
    let (uid, gid) = groups
        .uid_gid()
        .map_err(|_| WriteRefusal::GroupLookupFailed)?;
    let other_write = mode & 0o002 != 0;
    let group_write = mode & 0o020 != 0;
    let require_owner = matches!(
        role,
        WriteRole::CleanupComponent | WriteRole::PrivateStore | WriteRole::CacheLeaf
    );
    if require_owner && owner_uid != uid {
        return Err(WriteRefusal::UnexpectedOwner);
    }
    if role == WriteRole::StorageAncestor && owner_uid != 0 && owner_uid != uid {
        return Err(WriteRefusal::UnexpectedOwner);
    }
    if other_write {
        return Err(WriteRefusal::OtherWrite);
    }
    if group_write {
        exclusive_group(owner_uid, owner_gid, uid, gid, groups)?;
    }
    Ok(())
}

fn exclusive_group(
    owner_uid: u32,
    owner_gid: u32,
    uid: u32,
    gid: u32,
    groups: &dyn GroupQuery,
) -> Result<(), WriteRefusal> {
    if owner_uid != uid || owner_gid != gid {
        return Err(WriteRefusal::SharedGroupWrite);
    }
    let name = groups
        .user_name()
        .map_err(|_| WriteRefusal::GroupLookupFailed)?;
    if name.is_empty() {
        return Err(WriteRefusal::GroupLookupFailed);
    }
    let census = groups
        .census(owner_gid)
        .map_err(|_| WriteRefusal::GroupLookupFailed)?;
    if !census.supplementary.is_empty() {
        return Err(WriteRefusal::SharedGroupWrite);
    }
    if census.primary_users.len() == 1 && census.primary_users[0] == name {
        Ok(())
    } else {
        Err(WriteRefusal::SharedGroupWrite)
    }
}

struct LookupCache {
    caller: Option<Result<Caller, GroupLookupError>>,
    groups: HashMap<u32, Result<GroupCensus, GroupLookupError>>,
}

pub struct SystemGroups;

impl GroupQuery for SystemGroups {
    fn uid_gid(&self) -> Result<(u32, u32), GroupLookupError> {
        Ok(unsafe { (libc::getuid(), libc::getgid()) })
    }

    fn user_name(&self) -> Result<String, GroupLookupError> {
        let mut cache = LOOKUP.lock().map_err(|_| GroupLookupError)?;
        if let Some(cached) = &cache.caller {
            return cached.clone().map(|caller| caller.name);
        }
        let caller = unsafe { lookup_caller() };
        cache.caller = Some(caller.clone());
        caller.map(|caller| caller.name)
    }

    fn census(&self, gid: u32) -> Result<GroupCensus, GroupLookupError> {
        let mut cache = LOOKUP.lock().map_err(|_| GroupLookupError)?;
        if let Some(cached) = cache.groups.get(&gid) {
            return cached.clone();
        }
        let census = unsafe { lookup_census(gid) };
        cache.groups.insert(gid, census.clone());
        census
    }
}

static LOOKUP: LazyLock<Mutex<LookupCache>> = LazyLock::new(|| {
    Mutex::new(LookupCache {
        caller: None,
        groups: HashMap::new(),
    })
});

unsafe fn lookup_caller() -> Result<Caller, GroupLookupError> {
    let uid = libc::getuid();
    let gid = libc::getgid();
    let pw = libc::getpwuid(uid);
    if pw.is_null() {
        return Err(GroupLookupError);
    }
    let name = CStr::from_ptr((*pw).pw_name)
        .to_str()
        .map_err(|_| GroupLookupError)?
        .to_owned();
    if name.is_empty() {
        return Err(GroupLookupError);
    }
    Ok(Caller { uid, gid, name })
}

unsafe fn lookup_census(gid: u32) -> Result<GroupCensus, GroupLookupError> {
    let group = libc::getgrgid(gid);
    if group.is_null() || (*group).gr_mem.is_null() {
        return Err(GroupLookupError);
    }
    let mut supplementary = Vec::new();
    let mut member = (*group).gr_mem;
    while !(*member).is_null() {
        let name = CStr::from_ptr(*member)
            .to_str()
            .map_err(|_| GroupLookupError)?
            .to_owned();
        supplementary.push(name);
        member = member.add(1);
    }
    *libc::__errno_location() = 0;
    libc::setpwent();
    let mut primary_users = Vec::new();
    let primary = loop {
        *libc::__errno_location() = 0;
        let pw = libc::getpwent();
        if pw.is_null() {
            let failed = *libc::__errno_location() != 0;
            libc::endpwent();
            if failed {
                break Err(GroupLookupError);
            }
            break Ok(primary_users);
        }
        if (*pw).pw_gid == gid {
            let name = match CStr::from_ptr((*pw).pw_name).to_str() {
                Ok(name) => name.to_owned(),
                Err(_) => {
                    libc::endpwent();
                    break Err(GroupLookupError);
                }
            };
            primary_users.push(name);
        }
    };
    primary.map(|primary_users| GroupCensus {
        supplementary,
        primary_users,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        caller: Result<Caller, GroupLookupError>,
        census: Result<GroupCensus, GroupLookupError>,
    }

    impl GroupQuery for Fake {
        fn uid_gid(&self) -> Result<(u32, u32), GroupLookupError> {
            self.caller.clone().map(|caller| (caller.uid, caller.gid))
        }
        fn user_name(&self) -> Result<String, GroupLookupError> {
            self.caller.clone().map(|caller| caller.name)
        }
        fn census(&self, _: u32) -> Result<GroupCensus, GroupLookupError> {
            self.census.clone()
        }
    }

    fn caller() -> Caller {
        Caller {
            uid: 1000,
            gid: 1000,
            name: "only".into(),
        }
    }

    fn exclusive() -> Fake {
        Fake {
            caller: Ok(caller()),
            census: Ok(GroupCensus {
                supplementary: vec![],
                primary_users: vec!["only".into()],
            }),
        }
    }

    fn allow(
        role: WriteRole,
        uid: u32,
        gid: u32,
        mode: u32,
        groups: &Fake,
    ) -> Result<(), WriteRefusal> {
        stat_write_allowed(role, uid, gid, mode, groups)
    }

    #[test]
    fn private_group_write_is_accepted_and_other_write_is_not() {
        let groups = exclusive();
        assert!(allow(WriteRole::CleanupComponent, 1000, 1000, 0o775, &groups).is_ok());
        assert!(allow(WriteRole::PrivateStore, 1000, 1000, 0o775, &groups).is_ok());
        assert!(allow(WriteRole::CacheLeaf, 1000, 1000, 0o664, &groups).is_ok());
        assert!(allow(WriteRole::CleanupRoot, 1000, 1000, 0o775, &groups).is_ok());
        assert_eq!(
            allow(WriteRole::CleanupComponent, 1000, 1000, 0o777, &groups),
            Err(WriteRefusal::OtherWrite)
        );
        assert_eq!(
            allow(WriteRole::PrivateStore, 1000, 1000, 0o707, &groups),
            Err(WriteRefusal::OtherWrite)
        );
        assert_eq!(
            allow(WriteRole::CacheLeaf, 1000, 1000, 0o666, &groups),
            Err(WriteRefusal::OtherWrite)
        );
    }

    #[test]
    fn group_write_without_an_exclusive_census_is_refused() {
        let extra = Fake {
            caller: Ok(caller()),
            census: Ok(GroupCensus {
                supplementary: vec!["other".into()],
                primary_users: vec!["only".into()],
            }),
        };
        assert_eq!(
            allow(WriteRole::StorageAncestor, 1000, 1000, 0o775, &extra),
            Err(WriteRefusal::SharedGroupWrite)
        );
        let another_primary = Fake {
            caller: Ok(caller()),
            census: Ok(GroupCensus {
                supplementary: vec![],
                primary_users: vec!["only".into(), "guest".into()],
            }),
        };
        assert_eq!(
            allow(
                WriteRole::CleanupComponent,
                1000,
                1000,
                0o770,
                &another_primary
            ),
            Err(WriteRefusal::SharedGroupWrite)
        );
        let unknown = Fake {
            caller: Ok(caller()),
            census: Err(GroupLookupError),
        };
        assert_eq!(
            allow(WriteRole::PrivateStore, 1000, 1000, 0o775, &unknown),
            Err(WriteRefusal::GroupLookupFailed)
        );
        let no_caller = Fake {
            caller: Err(GroupLookupError),
            census: Ok(GroupCensus {
                supplementary: vec![],
                primary_users: vec!["only".into()],
            }),
        };
        assert_eq!(
            allow(WriteRole::CacheLeaf, 1000, 1000, 0o664, &no_caller),
            Err(WriteRefusal::GroupLookupFailed)
        );
    }

    #[test]
    fn group_write_requires_the_caller_uid_and_gid() {
        let groups = exclusive();
        assert_eq!(
            allow(WriteRole::CleanupComponent, 1000, 50, 0o775, &groups),
            Err(WriteRefusal::SharedGroupWrite)
        );
        assert_eq!(
            allow(WriteRole::CleanupComponent, 7, 1000, 0o775, &groups),
            Err(WriteRefusal::UnexpectedOwner)
        );
        assert_eq!(
            allow(WriteRole::CleanupRoot, 0, 0, 0o775, &groups),
            Err(WriteRefusal::SharedGroupWrite)
        );
        assert!(allow(WriteRole::CleanupRoot, 0, 0, 0o755, &groups).is_ok());
        assert!(allow(WriteRole::CleanupComponent, 1000, 1000, 0o755, &groups).is_ok());
        assert_eq!(
            allow(WriteRole::CleanupComponent, 0, 0, 0o755, &groups),
            Err(WriteRefusal::UnexpectedOwner)
        );
    }

    #[test]
    fn root_owned_sticky_ancestors_stay_allowed_and_modes_without_group_write_skip_the_census() {
        let broken = Fake {
            caller: Err(GroupLookupError),
            census: Err(GroupLookupError),
        };
        assert!(allow(WriteRole::StorageAncestor, 0, 0, 0o1777, &broken).is_ok());
        assert_eq!(
            allow(WriteRole::CleanupRoot, 0, 0, 0o1777, &broken),
            Err(WriteRefusal::GroupLookupFailed)
        );
        assert!(allow(WriteRole::StorageAncestor, 1000, 1000, 0o755, &broken).is_err());
        let groups = exclusive();
        assert!(allow(WriteRole::PrivateStore, 1000, 1000, 0o700, &groups).is_ok());
        assert!(allow(WriteRole::StorageAncestor, 0, 0, 0o755, &groups).is_ok());
        let nameless = Fake {
            caller: Ok(Caller {
                uid: 1000,
                gid: 1000,
                name: String::new(),
            }),
            census: Ok(GroupCensus {
                supplementary: vec![],
                primary_users: vec!["only".into()],
            }),
        };
        assert!(allow(WriteRole::PrivateStore, 1000, 1000, 0o700, &nameless).is_ok());
        assert_eq!(
            allow(WriteRole::PrivateStore, 1000, 1000, 0o775, &nameless),
            Err(WriteRefusal::GroupLookupFailed)
        );
    }
}
