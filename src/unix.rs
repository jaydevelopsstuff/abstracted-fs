use bitflags::{bitflags, Flags};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct UnixFilePermissionFlags(u32);

bitflags! {
    impl UnixFilePermissionFlags: u32 {
        const OWNER_READ = 0o400;
        const OWNER_WRITE = 0o200;
        const OWNER_EXEC = 0o100;
        const GROUP_READ = 0o40;
        const GROUP_WRITE = 0o20;
        const GROUP_EXEC = 0o10;
        const OTHER_READ = 0o4;
        const OTHER_WRITE = 0o2;
        const OTHER_EXEC = 0o1;
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct UnixFilePermissions {
    pub owner: UnixFileActions,
    pub group: UnixFileActions,
    pub other: UnixFileActions,
}

impl From<UnixFilePermissionFlags> for UnixFilePermissions {
    fn from(value: UnixFilePermissionFlags) -> Self {
        Self {
            owner: UnixFileActions {
                read: value.contains(UnixFilePermissionFlags::OWNER_READ),
                write: value.contains(UnixFilePermissionFlags::OWNER_WRITE),
                execute: value.contains(UnixFilePermissionFlags::OWNER_EXEC),
            },
            group: UnixFileActions {
                read: value.contains(UnixFilePermissionFlags::GROUP_READ),
                write: value.contains(UnixFilePermissionFlags::OWNER_WRITE),
                execute: value.contains(UnixFilePermissionFlags::OWNER_EXEC),
            },
            other: UnixFileActions {
                read: value.contains(UnixFilePermissionFlags::OTHER_READ),
                write: value.contains(UnixFilePermissionFlags::OTHER_WRITE),
                execute: value.contains(UnixFilePermissionFlags::OWNER_EXEC),
            },
        }
    }
}

impl From<UnixFilePermissions> for UnixFilePermissionFlags {
    fn from(permissions: UnixFilePermissions) -> Self {
        let mut flags = UnixFilePermissionFlags::empty();

        flags.set(Self::OWNER_READ, permissions.owner.read);
        flags.set(Self::OWNER_WRITE, permissions.owner.write);
        flags.set(Self::OWNER_EXEC, permissions.owner.execute);
        flags.set(Self::GROUP_READ, permissions.group.read);
        flags.set(Self::GROUP_WRITE, permissions.group.write);
        flags.set(Self::GROUP_EXEC, permissions.group.execute);
        flags.set(Self::OTHER_READ, permissions.other.read);
        flags.set(Self::OTHER_WRITE, permissions.other.write);
        flags.set(Self::OTHER_EXEC, permissions.other.execute);

        flags
    }
}

impl From<u32> for UnixFilePermissions {
    fn from(mode: u32) -> Self {
        UnixFilePermissionFlags::from_bits_truncate(mode).into()
    }
}

impl From<UnixFilePermissions> for u32 {
    fn from(permissions: UnixFilePermissions) -> Self {
        UnixFilePermissionFlags::from(permissions).bits()
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct UnixFileActions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}
