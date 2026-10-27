#[cfg(windows)]
pub fn attach() -> anyhow::Result<()> {
    use windows_sys::Win32::{
        Foundation::*,
        Storage::FileSystem::{FILE_TYPE_DISK, FILE_TYPE_PIPE, GetFileType},
        System::Console::*,
    };
    unsafe {
        let handles = [STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE].map(|kind| {
            let handle = GetStdHandle(kind);
            let redirected = !handle.is_null()
                && handle != INVALID_HANDLE_VALUE
                && matches!(GetFileType(handle), FILE_TYPE_DISK | FILE_TYPE_PIPE);
            (kind, handle, redirected)
        });
        if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() != Some(ERROR_ACCESS_DENIED as i32)
                && !handles[1].2
                && !handles[2].2
            {
                anyhow::bail!("cli: attaching parent console: {error}");
            }
        }
        for (kind, handle, redirected) in handles {
            if redirected && SetStdHandle(kind, handle) == 0 {
                anyhow::bail!(
                    "cli: restoring redirected console handle: {}",
                    std::io::Error::last_os_error()
                );
            }
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn attach() -> anyhow::Result<()> {
    Ok(())
}
