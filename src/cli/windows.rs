use std::{
    cell::RefCell,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    os::windows::{
        fs::{MetadataExt, OpenOptionsExt},
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::{Path, PathBuf},
    ptr,
    time::{Duration, Instant},
};

use anyhow::{Context, ensure};
use windows_sys::Win32::{
    Foundation::*,
    Storage::FileSystem::*,
    System::{Pipes::*, Threading::*},
};

mod overlap;
mod security;
use overlap::Overlap;

pub const RESPONSE_WAIT: Duration = Duration::from_secs(10);

pub fn registry() -> anyhow::Result<PathBuf> {
    let root = dirs::data_local_dir()
        .context("cli: no local application-data directory")?
        .join("luna-terminal-control");
    let descriptor = security::Descriptor::current()?;
    let attributes = descriptor.attributes();
    let text = security::wide(root.as_os_str());
    if unsafe { CreateDirectoryW(text.as_ptr(), &attributes) } == 0 {
        let error = io::Error::last_os_error();
        ensure!(
            error.raw_os_error() == Some(ERROR_ALREADY_EXISTS as i32),
            "cli: creating runtime registry: {error}"
        );
    }
    check_directory(&root)?;
    Ok(root)
}

pub fn check_directory(root: &Path) -> anyhow::Result<()> {
    let metadata = fs::symlink_metadata(root).context("cli: reading runtime registry metadata")?;
    ensure!(
        metadata.is_dir() && metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0,
        "cli: registry is not a real directory"
    );
    security::check(root)
}

fn check_record(path: &Path) -> anyhow::Result<()> {
    check_directory(path.parent().context("cli: endpoint has no parent")?)?;
    let metadata = fs::symlink_metadata(path).context("cli: reading endpoint metadata")?;
    ensure!(
        metadata.is_file()
            && metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0
            && metadata.len() <= 20,
        "cli: invalid endpoint record"
    );
    security::check(path)
}

pub fn discover(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    check_directory(root)?;
    let mut endpoints = Vec::new();
    for entry in fs::read_dir(root).context("cli: reading runtime registry")? {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "pipe") {
            check_record(&path)?;
            endpoints.push(path);
        }
    }
    endpoints.sort();
    Ok(endpoints)
}

fn name(path: &Path) -> anyhow::Result<Vec<u16>> {
    let instance = path
        .file_stem()
        .and_then(|name| name.to_str())
        .context("cli: invalid endpoint name")?;
    ensure!(
        !instance.is_empty()
            && instance.len() <= 64
            && instance
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-'),
        "cli: unsafe endpoint name"
    );
    Ok(security::wide(std::ffi::OsStr::new(&format!(
        r"\\.\pipe\luna-terminal-{}-{instance}",
        security::user()?
    ))))
}

struct Waiting {
    handle: Option<OwnedHandle>,
    operation: Overlap,
    complete: bool,
}

impl Waiting {
    fn new(path: &Path, first: bool) -> anyhow::Result<Self> {
        let name = name(path)?;
        let descriptor = security::Descriptor::current()?;
        let mut mode = PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED;
        if first {
            mode |= FILE_FLAG_FIRST_PIPE_INSTANCE;
        }
        let raw = unsafe {
            CreateNamedPipeW(
                name.as_ptr(),
                mode,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                32,
                65536,
                65536,
                2000,
                &descriptor.attributes(),
            )
        };
        ensure!(
            raw != INVALID_HANDLE_VALUE,
            "cli: creating control pipe: {}",
            io::Error::last_os_error()
        );
        let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
        let mut operation = Overlap::new()?;
        let result = unsafe { ConnectNamedPipe(handle.as_raw_handle(), operation.pointer()) };
        let complete = if result != 0 {
            true
        } else {
            let error = io::Error::last_os_error();
            ensure!(
                matches!(error.raw_os_error(), Some(code) if code == ERROR_IO_PENDING as i32 || code == ERROR_PIPE_CONNECTED as i32),
                "cli: listening for pipe connection: {error}"
            );
            error.raw_os_error() == Some(ERROR_PIPE_CONNECTED as i32)
        };
        Ok(Self {
            handle: Some(handle),
            operation,
            complete,
        })
    }
}
impl Drop for Waiting {
    fn drop(&mut self) {
        if !self.complete
            && let Some(handle) = &self.handle
        {
            self.operation.cancel(handle);
        }
    }
}

pub struct Listener {
    waiting: RefCell<Waiting>,
    path: PathBuf,
}
impl Listener {
    pub fn bind(root: &Path, instance: &str) -> anyhow::Result<Self> {
        check_directory(root)?;
        let path = root.join(format!("{instance}.pipe"));
        let waiting = Waiting::new(&path, true)?;
        let mut record = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .context("cli: publishing pipe record")?;
        write!(record, "{}", std::process::id()).context("cli: writing pipe record")?;
        security::check(&path)?;
        log::info!("cli: listening on pipe {}", path.display());
        Ok(Self {
            waiting: RefCell::new(waiting),
            path,
        })
    }

    pub fn accept(&self) -> anyhow::Result<Option<Connection>> {
        let mut waiting = self.waiting.borrow_mut();
        if !waiting.complete
            && !waiting
                .operation
                .ready(waiting.handle.as_ref().expect("waiting pipe owns a handle"))?
        {
            return Ok(None);
        }
        waiting.complete = true;
        let next = Waiting::new(&self.path, false)?;
        let connected = waiting.handle.take().expect("connected pipe owns a handle");
        *waiting = next;
        Ok(Some(Connection {
            handle: connected,
            deadline: Instant::now() + RESPONSE_WAIT,
        }))
    }
}
impl Drop for Listener {
    fn drop(&mut self) {
        if let Err(error) = check_record(&self.path)
            .and_then(|()| fs::remove_file(&self.path).context("cli: removing pipe record"))
        {
            log::error!("cli: cleaning up pipe record: {error:#}");
        }
    }
}

pub struct Connection {
    handle: OwnedHandle,
    deadline: Instant,
}
impl io::Read for Connection {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        overlap::read(&self.handle, buffer, self.deadline)
    }
}
impl io::Write for Connection {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        overlap::write(&self.handle, buffer, self.deadline)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn connect(path: &Path) -> anyhow::Result<Option<Connection>> {
    check_record(path)?;
    let name = name(path)?;
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let raw = unsafe {
            CreateFileW(
                name.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED,
                ptr::null_mut(),
            )
        };
        if raw != INVALID_HANDLE_VALUE {
            let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
            let mut pid = 0;
            ensure!(
                unsafe { GetNamedPipeServerProcessId(handle.as_raw_handle(), &mut pid) } != 0,
                "cli: reading pipe server PID: {}",
                io::Error::last_os_error()
            );
            let process = process(pid)?.context("cli: pipe server exited during connection")?;
            ensure!(
                security::process_user(process.as_raw_handle())? == security::user()?,
                "cli: pipe server belongs to another user"
            );
            return Ok(Some(Connection {
                handle,
                deadline: Instant::now() + RESPONSE_WAIT,
            }));
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() == Some(ERROR_FILE_NOT_FOUND as i32) {
            let pid: u32 = fs::read_to_string(path)
                .context("cli: reading stale endpoint PID")?
                .parse()
                .context("cli: invalid endpoint PID")?;
            if process(pid)?.is_none() {
                check_record(path)?;
                fs::remove_file(path).context("cli: removing stale pipe record")?;
                return Ok(None);
            }
            anyhow::bail!("cli: live application is not accepting pipe connections");
        }
        ensure!(
            error.raw_os_error() == Some(ERROR_PIPE_BUSY as i32),
            "cli: connecting to control pipe: {error}"
        );
        let wait = deadline
            .checked_duration_since(Instant::now())
            .context("cli: timed out waiting for control pipe")?;
        let milliseconds = wait.as_millis().clamp(1, 2000) as u32;
        if unsafe { WaitNamedPipeW(name.as_ptr(), milliseconds) } == 0 {
            anyhow::bail!(
                "cli: waiting for control pipe: {}",
                io::Error::last_os_error()
            );
        }
    }
}

fn process(pid: u32) -> anyhow::Result<Option<OwnedHandle>> {
    let raw = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE, 0, pid) };
    if raw.is_null() {
        let error = io::Error::last_os_error();
        if error.raw_os_error() == Some(ERROR_INVALID_PARAMETER as i32) {
            return Ok(None);
        }
        return Err(error).context("cli: querying endpoint process");
    }
    let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
    if unsafe { WaitForSingleObject(handle.as_raw_handle(), 0) } == WAIT_OBJECT_0 {
        return Ok(None);
    }
    Ok(Some(handle))
}

pub fn bootstrap_lock(root: &Path, deadline: Instant) -> anyhow::Result<File> {
    check_directory(root)?;
    let path = root.join("bootstrap.lock");
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(&path)
        .context("cli: opening startup lock")?;
    ensure!(
        lock.metadata()?.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0,
        "cli: startup lock is a reparse point"
    );
    security::check(&path)?;
    loop {
        match lock.try_lock() {
            Ok(()) => return Ok(lock),
            Err(std::fs::TryLockError::WouldBlock) => {
                ensure!(
                    Instant::now() < deadline,
                    "cli: timed out waiting for GUI startup lock"
                );
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(error) => return Err(error).context("cli: locking GUI startup"),
        }
    }
}
