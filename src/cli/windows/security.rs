use std::{
    ffi::c_void,
    io,
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::Path,
    ptr,
};

use anyhow::{Context, ensure};
use windows_sys::Win32::{
    Foundation::{HANDLE, LocalFree},
    Security::{Authorization::*, *},
    Storage::FileSystem::FILE_ALL_ACCESS,
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

pub fn wide(value: &std::ffi::OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}

pub fn user() -> anyhow::Result<String> {
    process_user(unsafe { GetCurrentProcess() })
}

pub fn process_user(process: HANDLE) -> anyhow::Result<String> {
    unsafe {
        let mut token = ptr::null_mut();
        ensure!(
            OpenProcessToken(process, TOKEN_QUERY, &mut token) != 0,
            "cli: opening process token: {}",
            io::Error::last_os_error()
        );
        let token = OwnedHandle::from_raw_handle(token);
        let mut size = 0;
        GetTokenInformation(
            token.as_raw_handle(),
            TokenUser,
            ptr::null_mut(),
            0,
            &mut size,
        );
        ensure!(
            size > 0,
            "cli: reading token size: {}",
            io::Error::last_os_error()
        );
        let mut storage = vec![0usize; (size as usize).div_ceil(std::mem::size_of::<usize>())];
        ensure!(
            GetTokenInformation(
                token.as_raw_handle(),
                TokenUser,
                storage.as_mut_ptr().cast(),
                size,
                &mut size
            ) != 0,
            "cli: reading token user: {}",
            io::Error::last_os_error()
        );
        sid_string((*(storage.as_ptr().cast::<TOKEN_USER>())).User.Sid)
    }
}

fn sid_string(sid: PSID) -> anyhow::Result<String> {
    unsafe {
        let mut value = ptr::null_mut();
        ensure!(
            ConvertSidToStringSidW(sid, &mut value) != 0,
            "cli: encoding user SID: {}",
            io::Error::last_os_error()
        );
        let allocation = Allocation(value.cast());
        let mut length = 0;
        while *value.add(length) != 0 {
            length += 1;
        }
        let result = String::from_utf16(std::slice::from_raw_parts(value, length))
            .context("cli: invalid user SID text");
        drop(allocation);
        result
    }
}

struct Allocation(*mut c_void);
impl Drop for Allocation {
    fn drop(&mut self) {
        unsafe {
            LocalFree(self.0);
        }
    }
}

pub struct Descriptor(Allocation);
impl Descriptor {
    pub fn current() -> anyhow::Result<Self> {
        let sid = user()?;
        let text = wide(std::ffi::OsStr::new(&format!(
            "O:{sid}D:P(A;OICI;FA;;;{sid})"
        )));
        unsafe {
            let mut descriptor = ptr::null_mut();
            ensure!(
                ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    text.as_ptr(),
                    SDDL_REVISION_1,
                    &mut descriptor,
                    ptr::null_mut()
                ) != 0,
                "cli: building current-user security descriptor: {}",
                io::Error::last_os_error()
            );
            Ok(Self(Allocation(descriptor)))
        }
    }

    pub fn attributes(&self) -> SECURITY_ATTRIBUTES {
        SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: self.0.0,
            bInheritHandle: 0,
        }
    }
}

pub fn check(path: &Path) -> anyhow::Result<()> {
    let text = wide(path.as_os_str());
    unsafe {
        let mut owner = ptr::null_mut();
        let mut acl: *mut ACL = ptr::null_mut();
        let mut descriptor = ptr::null_mut();
        let result = GetNamedSecurityInfoW(
            text.as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner,
            ptr::null_mut(),
            &mut acl,
            ptr::null_mut(),
            &mut descriptor,
        );
        ensure!(
            result == 0,
            "cli: reading registry security: {}",
            io::Error::from_raw_os_error(result as i32)
        );
        let _descriptor = Allocation(descriptor);
        ensure!(
            !owner.is_null() && !acl.is_null(),
            "cli: registry has no owner or DACL"
        );
        let sid = user()?;
        ensure!(
            sid_string(owner)? == sid && (*acl).AceCount == 1,
            "cli: registry is not current-user-only: {}",
            path.display()
        );
        let mut entry = ptr::null_mut();
        ensure!(
            GetAce(acl, 0, &mut entry) != 0,
            "cli: reading registry access rule: {}",
            io::Error::last_os_error()
        );
        let entry = &*(entry.cast::<ACCESS_ALLOWED_ACE>());
        ensure!(
            entry.Header.AceType == 0 && entry.Mask & FILE_ALL_ACCESS == FILE_ALL_ACCESS,
            "cli: unexpected registry access rule"
        );
        let allowed = (&entry.SidStart as *const u32).cast_mut().cast();
        ensure!(
            sid_string(allowed)? == sid,
            "cli: registry permits another user"
        );
    }
    Ok(())
}
