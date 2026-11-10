use super::*;

#[cfg(unix)]
#[test]
fn open_regular_reads_a_regular_file() {
    use std::io::Read;

    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("artifact.bin");
    std::fs::write(&path, b"artifact bytes").expect("write artifact");

    let mut bytes = Vec::new();
    open_regular(&path)
        .expect("open regular file")
        .read_to_end(&mut bytes)
        .expect("read artifact");
    assert_eq!(bytes, b"artifact bytes");
}

#[cfg(unix)]
#[test]
fn open_regular_rejects_a_symlink_to_a_regular_file() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("target.bin");
    let link = dir.path().join("link.bin");
    std::fs::write(&target, b"target").expect("write target");
    symlink(&target, &link).expect("create symlink");

    assert!(open_regular(&link).is_err());
}

#[cfg(unix)]
#[test]
fn open_regular_rejects_a_directory() {
    let dir = tempfile::tempdir().expect("tempdir");

    assert!(open_regular(dir.path()).is_err());
}

#[cfg(unix)]
#[test]
fn open_regular_rejects_a_fifo_without_hanging() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::sync::mpsc;
    use std::time::Duration;

    let dir = tempfile::tempdir().expect("tempdir");
    let fifo = dir.path().join("artifact.fifo");
    let fifo_c = CString::new(fifo.as_os_str().as_bytes()).expect("FIFO path has no NUL");
    // mkfifo's mode argument.
    assert_eq!(
        unsafe {
            libc::mkfifo(fifo_c.as_ptr(), /*mode*/ 0o600)
        },
        0
    );

    let path = fifo;
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        sender
            .send(open_regular(&path).map(|_| ()))
            .expect("send result")
    });

    assert!(
        receiver
            .recv_timeout(Duration::from_secs(/*secs*/ 5))
            .expect("opening a FIFO must not block")
            .is_err()
    );
}

#[cfg(unix)]
#[test]
fn open_regular_reads_only_the_requested_prefix() {
    use std::io::Read;

    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("artifact.bin");
    std::fs::write(&path, b"abcdef").expect("write artifact");

    let mut bytes = Vec::new();
    open_regular(&path)
        .expect("open regular file")
        .take(/*limit*/ 3)
        .read_to_end(&mut bytes)
        .expect("read bounded prefix");
    assert_eq!(bytes, b"abc");
}

#[test]
fn read_to_string_accepts_exact_limit_and_rejects_one_byte_over() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("artifact.json");
    std::fs::write(&path, b"abcd").expect("write artifact");

    assert_eq!(read_to_string(&path, 4).expect("exact limit"), "abcd");
    assert_eq!(
        read_to_string(&path, 3)
            .expect_err("over-limit artifact")
            .kind(),
        std::io::ErrorKind::InvalidData
    );
}

#[test]
fn read_to_string_rejects_invalid_utf8() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("artifact.json");
    std::fs::write(&path, [0xff]).expect("write invalid artifact");

    assert_eq!(
        read_to_string(&path, 1)
            .expect_err("invalid UTF-8 artifact")
            .kind(),
        std::io::ErrorKind::InvalidData
    );
}

#[cfg(unix)]
#[test]
fn open_beneath_rejects_symlinked_ancestor_and_reads_nested_file() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::tempdir().expect("outside");
    std::fs::write(outside.path().join("f.txt"), b"secret").expect("write outside");
    std::fs::create_dir(dir.path().join("real")).expect("mkdir");
    std::fs::write(dir.path().join("real/f.txt"), b"ok").expect("write nested");
    symlink(outside.path(), dir.path().join("link")).expect("symlink dir");

    let text = read_to_string_beneath(dir.path(), Path::new("real/f.txt"), 16).expect("nested");
    assert_eq!(text, "ok");
    assert!(read_to_string_beneath(dir.path(), Path::new("link/f.txt"), 16).is_err());
    assert!(read_to_string_beneath(dir.path(), Path::new("../x"), 16).is_err());
    assert!(read_to_string_beneath(dir.path(), Path::new("real/f.txt"), 1).is_err());
}
