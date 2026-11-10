use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::io::Read;
use std::path::Path;

/// Opens a regular file and rejects symlinks on Unix without blocking on FIFOs.
pub(crate) fn open_regular(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;

        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "expected regular file",
        ));
    }
    Ok(file)
}

/// Reads one local artifact without allowing an unbounded allocation.
pub(crate) fn read_to_string(path: &Path, max_bytes: usize) -> io::Result<String> {
    let mut bytes = Vec::with_capacity(max_bytes.min(8192));
    open_regular(path)?
        .take(max_bytes as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > max_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "artifact exceeds the configured byte limit",
        ));
    }
    String::from_utf8(bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "artifact is not valid UTF-8"))
}

/// Opens `relative` under `root` one component at a time, refusing symlinks at
/// every component below `root` so an ancestor swap cannot redirect the read.
pub(crate) fn open_beneath(root: &Path, relative: &Path) -> io::Result<File> {
    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::os::fd::FromRawFd;
        use std::os::fd::OwnedFd;
        use std::os::unix::ffi::OsStrExt;

        let invalid = || io::Error::new(io::ErrorKind::InvalidInput, "path escapes root");
        let names: Vec<_> = relative
            .components()
            .map(|c| match c {
                std::path::Component::Normal(name) => Ok(name),
                _ => Err(invalid()),
            })
            .collect::<io::Result<_>>()?;
        if names.is_empty() {
            return Err(invalid());
        }
        let mut current: OwnedFd = File::open(root)?.into();
        let last = names.len() - 1;
        for (index, name) in names.iter().enumerate() {
            let c_name = CString::new(name.as_bytes()).map_err(|_| invalid())?;
            let flags = if index == last {
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC
            } else {
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_DIRECTORY | libc::O_CLOEXEC
            };
            use std::os::fd::AsRawFd;
            // SAFETY: `current` is a live descriptor and `c_name` is NUL-terminated.
            let fd = unsafe { libc::openat(current.as_raw_fd(), c_name.as_ptr(), flags) };
            if fd < 0 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: `fd` was just returned by a successful openat and is owned here.
            current = unsafe { OwnedFd::from_raw_fd(fd) };
        }
        let file = File::from(current);
        if !file.metadata()?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "expected regular file",
            ));
        }
        Ok(file)
    }
    #[cfg(not(unix))]
    {
        open_regular(&root.join(relative))
    }
}

/// Bounded UTF-8 read of `relative` under `root` via [`open_beneath`].
pub(crate) fn read_to_string_beneath(
    root: &Path,
    relative: &Path,
    max_bytes: usize,
) -> io::Result<String> {
    let mut bytes = Vec::with_capacity(max_bytes.min(8192));
    open_beneath(root, relative)?
        .take(max_bytes as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > max_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "artifact exceeds the configured byte limit",
        ));
    }
    String::from_utf8(bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "artifact is not valid UTF-8"))
}

#[cfg(test)]
#[path = "artifact_io_tests.rs"]
mod tests;
