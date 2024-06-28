const OWNER_READ: u32 = 0o400;
const OWNER_WRITE: u32 = 0o200;
const OWNER_EXEC: u32 = 0o100;
const GROUP_READ: u32 = 0o40;
const GROUP_WRITE: u32 = 0o20;
const GROUP_EXEC: u32 = 0o10;
const OTHER_READ: u32 = 0o4;
const OTHER_WRITE: u32 = 0o2;
const OTHER_EXEC: u32 = 0o1;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct UnixFilePermissions {
    pub owner: UnixFileActions,
    pub group: UnixFileActions,
    pub other: UnixFileActions,
}

impl From<u32> for UnixFilePermissions {
    fn from(value: u32) -> Self {
        Self {
            owner: UnixFileActions {
                read: (value & OWNER_READ) == OWNER_READ,
                write: (value & OWNER_WRITE) == OWNER_WRITE,
                execute: (value & OWNER_EXEC) == OWNER_EXEC,
            },
            group: UnixFileActions {
                read: (value & GROUP_READ) == GROUP_READ,
                write: (value & GROUP_WRITE) == GROUP_WRITE,
                execute: (value & GROUP_EXEC) == GROUP_EXEC,
            },
            other: UnixFileActions {
                read: (value & OTHER_READ) == OTHER_READ,
                write: (value & OTHER_WRITE) == OTHER_WRITE,
                execute: (value & OTHER_EXEC) == OTHER_EXEC,
            },
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct UnixFileActions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}
