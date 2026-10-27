use super::*;
use crate::cli::protocol::{Operation, Request, Response, read_frame, write_frame};
use std::{
    os::unix::fs::symlink,
    sync::atomic::{AtomicU64, Ordering},
};

struct Directory(PathBuf);

impl Directory {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "luna-cli-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(path)
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn private_socket_communicates_and_cleans_up() {
    let root = Directory::new();
    let listener = Listener::bind(&root.0, "instance").unwrap();
    let paths = discover(&root.0).unwrap();
    assert_eq!(paths.len(), 1);
    assert_eq!(fs::metadata(&paths[0]).unwrap().mode() & 0o777, 0o600);
    let mut client = connect(&paths[0]).unwrap().unwrap();
    let mut server = listener.accept().unwrap().unwrap();
    write_frame(&mut client, &Request::new(Operation::List {})).unwrap();
    let request: Request = read_frame(&mut server).unwrap();
    request.validate().unwrap();
    write_frame(&mut server, &Response::Applied {}).unwrap();
    assert!(matches!(
        read_frame::<Response>(&mut client).unwrap(),
        Response::Applied {}
    ));
    assert!(listener.accept().unwrap().is_none());
    drop(listener);
    assert!(discover(&root.0).unwrap().is_empty());
}

#[test]
fn unsafe_directories_and_symlinks_are_refused() {
    let root = Directory::new();
    let link = root.0.join("link");
    symlink(&root.0, &link).unwrap();
    assert!(check_directory(&link).is_err());
    fs::set_permissions(&root.0, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(Listener::bind(&root.0, "instance").is_err());
    assert!(discover(&root.0).is_err());
}

#[test]
fn discovery_refuses_fake_or_unsafe_endpoints() {
    let root = Directory::new();
    let path = root.0.join("fake.sock");
    fs::write(&path, "not a socket").unwrap();
    assert!(discover(&root.0).is_err());
    fs::remove_file(&path).unwrap();
    symlink("/tmp", &path).unwrap();
    assert!(discover(&root.0).is_err());
    fs::remove_file(&path).unwrap();
    let listener = Listener::bind(&root.0, "real").unwrap();
    fs::set_permissions(&listener.path, fs::Permissions::from_mode(0o666)).unwrap();
    assert!(connect(&listener.path).is_err());
    fs::set_permissions(&listener.path, fs::Permissions::from_mode(0o600)).unwrap();
}

#[test]
fn stale_socket_cleanup_does_not_remove_other_files() {
    let root = Directory::new();
    let path = root.0.join("stale.sock");
    let stale = UnixListener::bind(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    drop(stale);
    let other = root.0.join("keep");
    fs::write(&other, "keep").unwrap();
    assert!(connect(&path).unwrap().is_none());
    assert!(!path.exists());
    assert_eq!(fs::read_to_string(other).unwrap(), "keep");
}

#[test]
fn endpoint_names_cannot_escape_the_registry() {
    let root = Directory::new();
    for name in ["", "../escape", "/tmp/escape", "with space"] {
        assert!(Listener::bind(&root.0, name).is_err());
    }
}

#[test]
fn slow_readers_and_expired_connections_have_finite_deadlines() {
    use std::io::{Read, Write};
    let (reader, mut peer) = UnixStream::pair().unwrap();
    let mut connection = Connection::new(reader);
    connection.deadline = Instant::now() + Duration::from_millis(30);
    let started = Instant::now();
    assert!(connection.read_exact(&mut [0; 4]).is_err());
    assert!(started.elapsed() < Duration::from_secs(1));
    connection.deadline = Instant::now() - Duration::from_millis(1);
    peer.write_all(&[0; 4]).unwrap();
    assert_eq!(
        connection.read(&mut [0; 4]).unwrap_err().kind(),
        io::ErrorKind::TimedOut
    );
    assert_eq!(
        connection.write(b"late").unwrap_err().kind(),
        io::ErrorKind::TimedOut
    );
}
