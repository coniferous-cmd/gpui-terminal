use std::{
    fs::{self, DirBuilder, File, OpenOptions},
    io,
    os::unix::{
        fs::{DirBuilderExt, FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt},
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::{Context, ensure};

pub const RESPONSE_WAIT: Duration = Duration::from_secs(10);

pub fn registry() -> anyhow::Result<PathBuf> {
    let root = std::env::temp_dir().join(format!("luna-terminal-{}", user()));
    match DirBuilder::new().mode(0o700).create(&root) {
        Ok(()) => log::debug!("cli: created registry {}", root.display()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error).context("cli: creating runtime registry"),
    }
    check_directory(&root)?;
    Ok(root)
}

pub fn user() -> u32 {
    unsafe { libc::geteuid() }
}

pub fn check_directory(path: &Path) -> anyhow::Result<()> {
    let metadata = fs::symlink_metadata(path).context("cli: reading runtime registry metadata")?;
    ensure!(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        "cli: registry is not a real directory: {}",
        path.display()
    );
    ensure!(
        metadata.uid() == user(),
        "cli: registry has another owner: {}",
        path.display()
    );
    ensure!(
        metadata.mode() & 0o077 == 0,
        "cli: registry is accessible to other users: {}",
        path.display()
    );
    Ok(())
}

fn check_socket(path: &Path) -> anyhow::Result<fs::Metadata> {
    check_directory(
        path.parent()
            .context("cli: socket has no parent directory")?,
    )?;
    let metadata = fs::symlink_metadata(path).context("cli: reading socket metadata")?;
    ensure!(
        metadata.file_type().is_socket(),
        "cli: endpoint is not a socket: {}",
        path.display()
    );
    ensure!(
        metadata.uid() == user() && metadata.mode() & 0o077 == 0,
        "cli: unsafe socket ownership or permissions: {}",
        path.display()
    );
    Ok(metadata)
}

pub fn discover(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    check_directory(root)?;
    let mut endpoints = Vec::new();
    for entry in fs::read_dir(root).context("cli: reading runtime registry")? {
        let path = entry.context("cli: reading runtime registry entry")?.path();
        if path.extension().is_some_and(|ext| ext == "sock") {
            check_socket(&path)?;
            endpoints.push(path);
        }
    }
    endpoints.sort();
    Ok(endpoints)
}

pub struct Listener {
    listener: UnixListener,
    path: PathBuf,
    inode: u64,
}

impl Listener {
    pub fn bind(root: &Path, instance: &str) -> anyhow::Result<Self> {
        check_directory(root)?;
        ensure!(
            !instance.is_empty()
                && instance
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-'),
            "cli: unsafe endpoint name"
        );
        let path = root.join(format!("{instance}.sock"));
        let listener = UnixListener::bind(&path).context("cli: binding control socket")?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .context("cli: restricting control socket")?;
        listener
            .set_nonblocking(true)
            .context("cli: setting control listener nonblocking")?;
        let inode = check_socket(&path)?.ino();
        log::info!("cli: listening on {}", path.display());
        Ok(Self {
            listener,
            path,
            inode,
        })
    }

    pub fn accept(&self) -> anyhow::Result<Option<Connection>> {
        match self.listener.accept() {
            Ok((stream, _)) => {
                authorize(&stream)?;
                configure(&stream)?;
                Ok(Some(Connection::new(stream)))
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(error) => Err(error).context("cli: accepting control connection"),
        }
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        match check_socket(&self.path) {
            Ok(metadata) if metadata.ino() == self.inode => {
                if let Err(error) = fs::remove_file(&self.path) {
                    log::error!("cli: removing endpoint {}: {error}", self.path.display());
                }
            }
            Ok(_) => log::error!(
                "cli: endpoint replaced before cleanup: {}",
                self.path.display()
            ),
            Err(error) => log::error!("cli: endpoint cleanup refused: {error:#}"),
        }
    }
}

pub fn connect(path: &Path) -> anyhow::Result<Option<Connection>> {
    let original = check_socket(path)?;
    let socket = socket2::Socket::new(socket2::Domain::UNIX, socket2::Type::STREAM, None)
        .context("cli: creating client socket")?;
    let address = socket2::SockAddr::unix(path).context("cli: encoding socket address")?;
    match socket.connect_timeout(&address, Duration::from_secs(2)) {
        Ok(()) => {
            let stream = UnixStream::from(std::os::fd::OwnedFd::from(socket));
            authorize(&stream)?;
            configure(&stream)?;
            Ok(Some(Connection::new(stream)))
        }
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::ConnectionRefused | io::ErrorKind::NotFound
            ) =>
        {
            match fs::symlink_metadata(path) {
                Ok(current)
                    if current.ino() == original.ino()
                        && current.uid() == user()
                        && current.file_type().is_socket() =>
                {
                    fs::remove_file(path).context("cli: removing stale socket")?;
                    log::info!("cli: removed stale endpoint {}", path.display());
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                _ => anyhow::bail!(
                    "cli: endpoint changed during stale cleanup: {}",
                    path.display()
                ),
            }
            Ok(None)
        }
        Err(error) => Err(error).context("cli: connecting to running terminal"),
    }
}

pub struct Connection {
    stream: UnixStream,
    deadline: Instant,
}

impl Connection {
    fn new(stream: UnixStream) -> Self {
        Self {
            stream,
            deadline: Instant::now() + RESPONSE_WAIT,
        }
    }

    fn remaining(&self) -> io::Result<Duration> {
        self.deadline
            .checked_duration_since(Instant::now())
            .filter(|time| !time.is_zero())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::TimedOut,
                    "cli: connection deadline expired; operation outcome may be unknown",
                )
            })
    }

    fn wait(&self, events: libc::c_short) -> io::Result<()> {
        use std::os::fd::AsRawFd;
        loop {
            let wait = self
                .remaining()?
                .as_millis()
                .saturating_add(1)
                .min(i32::MAX as u128) as i32;
            let mut descriptor = libc::pollfd {
                fd: self.stream.as_raw_fd(),
                events,
                revents: 0,
            };
            let result = unsafe { libc::poll(&mut descriptor, 1, wait) };
            if result > 0 {
                return Ok(());
            }
            if result < 0 {
                let error = io::Error::last_os_error();
                if error.kind() != io::ErrorKind::Interrupted {
                    return Err(error);
                }
            }
        }
    }
}

impl io::Read for Connection {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        loop {
            self.wait(libc::POLLIN)?;
            match io::Read::read(&mut self.stream, buffer) {
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                result => return result,
            }
        }
    }
}

impl io::Write for Connection {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        loop {
            self.wait(libc::POLLOUT)?;
            match io::Write::write(&mut self.stream, buffer) {
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                result => return result,
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        io::Write::flush(&mut self.stream)
    }
}

fn configure(stream: &UnixStream) -> anyhow::Result<()> {
    stream
        .set_nonblocking(true)
        .context("cli: setting control connection nonblocking")?;
    Ok(())
}

fn authorize(stream: &UnixStream) -> anyhow::Result<()> {
    use std::os::fd::AsRawFd;
    #[cfg(target_os = "linux")]
    let peer = unsafe {
        let mut credentials: libc::ucred = std::mem::zeroed();
        let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        ensure!(
            libc::getsockopt(
                stream.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&mut credentials as *mut libc::ucred).cast(),
                &mut length
            ) == 0,
            "cli: reading peer credentials: {}",
            io::Error::last_os_error()
        );
        credentials.uid
    };
    #[cfg(target_os = "macos")]
    let peer = unsafe {
        let mut uid = 0;
        let mut gid = 0;
        ensure!(
            libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) == 0,
            "cli: reading peer credentials: {}",
            io::Error::last_os_error()
        );
        uid
    };
    ensure!(peer == user(), "cli: control peer belongs to another user");
    Ok(())
}

pub fn bootstrap_lock(root: &Path, deadline: Instant) -> anyhow::Result<File> {
    check_directory(root)?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(root.join("bootstrap.lock"))
        .context("cli: opening startup lock")?;
    let metadata = lock
        .metadata()
        .context("cli: reading startup lock metadata")?;
    ensure!(
        metadata.is_file() && metadata.uid() == user() && metadata.mode() & 0o077 == 0,
        "cli: unsafe startup lock"
    );
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

#[cfg(test)]
mod tests;
