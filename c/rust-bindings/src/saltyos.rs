// SaltyOS libc bindings
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Raw FFI type definitions and extern declarations matching besalt-c.
// Values are sourced from:
//   - lib/besalt/uapi/rust/consts.rs
//   - lib/besalt/c/src/errno.rs
//   - lib/besalt/c/src/unistd.rs (Stat struct)
//   - lib/besalt/c/src/dirent_impl.rs (Dirent struct)
//   - lib/besalt/c/src/pthread_impl.rs (pthread opaque types)
//   - lib/besalt/c/src/signal_impl.rs (Sigaction, Sigset)
//   - lib/besalt/c/src/time_impl.rs (Timespec, Timeval)
//   - lib/besalt/c/src/select_impl.rs (FdSet)
//   - lib/besalt/c/include/ headers

// In no_core builds (rustc-dep-of-std), bring Option into scope since the
// core prelude is not active.
#[cfg(feature = "rustc-dep-of-std")]
use core::option::Option::{self, None, Some};

pub type c_void = core::ffi::c_void;

// =========================================================================
// Primitive type aliases
// =========================================================================

pub type c_char = i8;
pub type c_schar = i8;
pub type c_uchar = u8;
pub type c_short = i16;
pub type c_ushort = u16;
pub type c_int = i32;
pub type c_uint = u32;
pub type c_long = i64;
pub type c_ulong = u64;
pub type c_longlong = i64;
pub type c_ulonglong = u64;
pub type c_float = f32;
pub type c_double = f64;
pub type c_size_t = usize;
pub type size_t = usize;
pub type ssize_t = isize;
pub type ptrdiff_t = isize;
pub type intptr_t = isize;
pub type uintptr_t = usize;
pub type off_t = i64;
pub type mode_t = u32;
pub type pid_t = i32;
pub type uid_t = u32;
pub type gid_t = u32;
pub type time_t = i64;
pub type clockid_t = i32;
pub type nfds_t = u32;
pub type ino_t = u64;
pub type dev_t = u64;
pub type nlink_t = u64;
pub type blksize_t = i64;
pub type blkcnt_t = i64;
pub type suseconds_t = i64;
pub type pthread_t = u64;
pub type pthread_key_t = u32;
pub type socklen_t = u32;
pub type sa_family_t = u16;
pub type in_addr_t = u32;
pub type in_port_t = u16;
pub type sighandler_t = usize;
pub type sig_atomic_t = i32;
pub type wchar_t = i32;
pub type wint_t = u32;
pub type rlim_t = u64;

// FILE is opaque
pub enum FILE {}

// DIR is opaque
pub enum DIR {}

// =========================================================================
// Errno constants (from lib/besalt/c/src/errno.rs — Linux numbering)
// =========================================================================

pub const EPERM: c_int = 1;
pub const ENOENT: c_int = 2;
pub const ESRCH: c_int = 3;
pub const EINTR: c_int = 4;
pub const EIO: c_int = 5;
pub const ENXIO: c_int = 6;
pub const E2BIG: c_int = 7;
pub const ENOEXEC: c_int = 8;
pub const EBADF: c_int = 9;
pub const ECHILD: c_int = 10;
pub const EAGAIN: c_int = 11;
pub const ENOMEM: c_int = 12;
pub const EACCES: c_int = 13;
pub const EFAULT: c_int = 14;
pub const EBUSY: c_int = 16;
pub const EEXIST: c_int = 17;
pub const EXDEV: c_int = 18;
pub const ENODEV: c_int = 19;
pub const ENOTDIR: c_int = 20;
pub const EISDIR: c_int = 21;
pub const EINVAL: c_int = 22;
pub const ENFILE: c_int = 23;
pub const EMFILE: c_int = 24;
pub const ENOTTY: c_int = 25;
pub const EFBIG: c_int = 27;
pub const ENOSPC: c_int = 28;
pub const ESPIPE: c_int = 29;
pub const EROFS: c_int = 30;
pub const EMLINK: c_int = 31;
pub const EPIPE: c_int = 32;
pub const EDOM: c_int = 33;
pub const ERANGE: c_int = 34;
pub const EDEADLK: c_int = 35;
pub const ENAMETOOLONG: c_int = 36;
pub const ENOLCK: c_int = 37;
pub const ENOSYS: c_int = 38;
pub const ENOTEMPTY: c_int = 39;
pub const ELOOP: c_int = 40;
pub const EWOULDBLOCK: c_int = EAGAIN;
pub const ENODATA: c_int = 61;
pub const ETIME: c_int = 62;
pub const EPROTO: c_int = 71;
pub const EMULTIHOP: c_int = 72;
pub const EBADMSG: c_int = 74;
pub const EOVERFLOW: c_int = 75;
pub const EILSEQ: c_int = 84;
pub const ENOTSOCK: c_int = 88;
pub const EOPNOTSUPP: c_int = 95;
pub const ENOTSUP: c_int = EOPNOTSUPP;
pub const EAFNOSUPPORT: c_int = 97;
pub const EADDRINUSE: c_int = 98;
pub const ECONNRESET: c_int = 104;
pub const ENOBUFS: c_int = 105;
pub const EISCONN: c_int = 106;
pub const ENOTCONN: c_int = 107;
pub const ETIMEDOUT: c_int = 110;
pub const ECONNREFUSED: c_int = 111;
pub const EALREADY: c_int = 114;
pub const EINPROGRESS: c_int = 115;
pub const ECANCELED: c_int = 125;
pub const EOWNERDEAD: c_int = 130;
pub const ENOTRECOVERABLE: c_int = 131;
// Aliases used by some POSIX code
pub const ECONNABORTED: c_int = 103;
pub const EHOSTUNREACH: c_int = 113;
pub const ENETUNREACH: c_int = 101;
pub const EPROTONOSUPPORT: c_int = 93;
pub const EDESTADDRREQ: c_int = 89;
pub const EMSGSIZE: c_int = 90;

// =========================================================================
// File I/O constants (from uapi/rust/consts.rs and C headers)
// =========================================================================

pub const O_RDONLY: c_int = 0x0000;
pub const O_WRONLY: c_int = 0x0001;
pub const O_RDWR: c_int = 0x0002;
pub const O_ACCMODE: c_int = 0x0003;
pub const O_CREAT: c_int = 0x0040;
pub const O_EXCL: c_int = 0x0080;
pub const O_NOCTTY: c_int = 0x0100;
pub const O_TRUNC: c_int = 0x0200;
pub const O_APPEND: c_int = 0x0400;
pub const O_NONBLOCK: c_int = 0x0800;
pub const O_DIRECTORY: c_int = 0x10000;
pub const O_NOFOLLOW: c_int = 0x20000;
pub const O_CLOEXEC: c_int = 0x80000i32;

pub const SEEK_SET: c_int = 0;
pub const SEEK_CUR: c_int = 1;
pub const SEEK_END: c_int = 2;

// AT_* flags (from uapi/rust/consts.rs)
pub const AT_FDCWD: c_int = -100;
pub const AT_SYMLINK_NOFOLLOW: c_int = 0x100;
pub const AT_REMOVEDIR: c_int = 0x200;
pub const AT_SYMLINK_FOLLOW: c_int = 0x400;
pub const AT_EMPTY_PATH: c_int = 0x1000;

// =========================================================================
// Socket constants (from uapi/rust/consts.rs and C headers)
// =========================================================================

pub const AF_UNSPEC: c_int = 0;
pub const AF_UNIX: c_int = 1;
pub const AF_LOCAL: c_int = AF_UNIX;
pub const AF_INET: c_int = 2;
pub const AF_INET6: c_int = 10;

pub const SOCK_STREAM: c_int = 1;
pub const SOCK_DGRAM: c_int = 2;
pub const SOCK_NONBLOCK: c_int = O_NONBLOCK;
pub const SOCK_CLOEXEC: c_int = 0x80000i32;

pub const SOL_SOCKET: c_int = 1;
pub const SO_REUSEADDR: c_int = 2;
pub const SO_ERROR: c_int = 4;
pub const SO_KEEPALIVE: c_int = 9;
pub const SO_SNDBUF: c_int = 7;
pub const SO_RCVBUF: c_int = 8;
pub const SO_TYPE: c_int = 3;
pub const SO_REUSEPORT: c_int = 15;
pub const SO_BROADCAST: c_int = 6;
pub const SO_LINGER: c_int = 13;
pub const SO_RCVTIMEO: c_int = 20;
pub const SO_SNDTIMEO: c_int = 21;
pub const SO_ACCEPTCONN: c_int = 30;

pub const MSG_DONTWAIT: c_int = 0x40;
pub const MSG_NOSIGNAL: c_int = 0x4000;
pub const MSG_PEEK: c_int = 0x02;

pub const SHUT_RD: c_int = 0;
pub const SHUT_WR: c_int = 1;
pub const SHUT_RDWR: c_int = 2;

pub const IPPROTO_TCP: c_int = 6;
pub const IPPROTO_UDP: c_int = 17;
pub const IPPROTO_IP: c_int = 0;

pub const TCP_NODELAY: c_int = 1;
pub const TCP_KEEPIDLE: c_int = 4;
pub const TCP_KEEPINTVL: c_int = 5;
pub const TCP_KEEPCNT: c_int = 6;

pub const INADDR_ANY: in_addr_t = 0;
pub const INADDR_LOOPBACK: in_addr_t = 0x7f000001;
pub const INADDR_BROADCAST: in_addr_t = 0xffffffff;

pub const SCM_RIGHTS: c_int = 1;

pub const INET_ADDRSTRLEN: usize = 16;
pub const INET6_ADDRSTRLEN: usize = 46;

// =========================================================================
// Stat type bits (standard POSIX values)
// =========================================================================

pub const S_IFMT: mode_t = 0o170000;
pub const S_IFSOCK: mode_t = 0o140000;
pub const S_IFLNK: mode_t = 0o120000;
pub const S_IFREG: mode_t = 0o100000;
pub const S_IFBLK: mode_t = 0o060000;
pub const S_IFDIR: mode_t = 0o040000;
pub const S_IFCHR: mode_t = 0o020000;
pub const S_IFIFO: mode_t = 0o010000;

pub const S_IRWXU: mode_t = 0o700;
pub const S_IRUSR: mode_t = 0o400;
pub const S_IWUSR: mode_t = 0o200;
pub const S_IXUSR: mode_t = 0o100;
pub const S_IRWXG: mode_t = 0o070;
pub const S_IRGRP: mode_t = 0o040;
pub const S_IWGRP: mode_t = 0o020;
pub const S_IXGRP: mode_t = 0o010;
pub const S_IRWXO: mode_t = 0o007;
pub const S_IROTH: mode_t = 0o004;
pub const S_IWOTH: mode_t = 0o002;
pub const S_IXOTH: mode_t = 0o001;
pub const S_ISUID: mode_t = 0o4000;
pub const S_ISGID: mode_t = 0o2000;
pub const S_ISVTX: mode_t = 0o1000;

// =========================================================================
// Mmap constants (from uapi/rust/consts.rs)
// =========================================================================

pub const PROT_NONE: c_int = 0x0;
pub const PROT_READ: c_int = 0x1;
pub const PROT_WRITE: c_int = 0x2;
pub const PROT_EXEC: c_int = 0x4;

pub const MAP_SHARED: c_int = 0x01;
pub const MAP_PRIVATE: c_int = 0x02;
pub const MAP_FIXED: c_int = 0x10;
pub const MAP_ANONYMOUS: c_int = 0x20;
pub const MAP_ANON: c_int = MAP_ANONYMOUS;
pub const MAP_FAILED: *mut c_void = !0usize as *mut c_void;

pub const MADV_DONTNEED: c_int = 4;
pub const MADV_FREE: c_int = 8;

// =========================================================================
// Poll/epoll constants (from uapi/rust/consts.rs)
// =========================================================================

pub const POLLIN: c_short = 0x001;
pub const POLLPRI: c_short = 0x002;
pub const POLLOUT: c_short = 0x004;
pub const POLLERR: c_short = 0x008;
pub const POLLHUP: c_short = 0x010;
pub const POLLNVAL: c_short = 0x020;
pub const POLLRDNORM: c_short = 0x040;
pub const POLLWRNORM: c_short = 0x100;

pub const EPOLLIN: c_int = 0x001;
pub const EPOLLPRI: c_int = 0x002;
pub const EPOLLOUT: c_int = 0x004;
pub const EPOLLERR: c_int = 0x008;
pub const EPOLLHUP: c_int = 0x010;
pub const EPOLLET: c_int = 1 << 31;
pub const EPOLLONESHOT: c_int = 1 << 30;
pub const EPOLL_CTL_ADD: c_int = 1;
pub const EPOLL_CTL_DEL: c_int = 2;
pub const EPOLL_CTL_MOD: c_int = 3;
pub const EPOLL_CLOEXEC: c_int = O_CLOEXEC;

// =========================================================================
// Signal constants (from C headers and signal_impl.rs)
// =========================================================================

pub const SIGHUP: c_int = 1;
pub const SIGINT: c_int = 2;
pub const SIGQUIT: c_int = 3;
pub const SIGILL: c_int = 4;
pub const SIGTRAP: c_int = 5;
pub const SIGABRT: c_int = 6;
pub const SIGBUS: c_int = 7;
pub const SIGFPE: c_int = 8;
pub const SIGKILL: c_int = 9;
pub const SIGUSR1: c_int = 10;
pub const SIGSEGV: c_int = 11;
pub const SIGUSR2: c_int = 12;
pub const SIGPIPE: c_int = 13;
pub const SIGALRM: c_int = 14;
pub const SIGTERM: c_int = 15;
pub const SIGSTKFLT: c_int = 16;
pub const SIGCHLD: c_int = 17;
pub const SIGCONT: c_int = 18;
pub const SIGSTOP: c_int = 19;
pub const SIGTSTP: c_int = 20;
pub const SIGTTIN: c_int = 21;
pub const SIGTTOU: c_int = 22;
pub const SIGWINCH: c_int = 28;
pub const SIGINFO: c_int = 29;
pub const NSIG: c_int = 32;
pub const _NSIG: c_int = NSIG;

pub const SIG_DFL: sighandler_t = 0;
pub const SIG_IGN: sighandler_t = 1;
pub const SIG_ERR: sighandler_t = !0;

// SA_* flags (from signal_impl.rs)
pub const SA_NOCLDSTOP: c_int = 0x00000001;
pub const SA_NOCLDWAIT: c_int = 0x00000002;
pub const SA_SIGINFO: c_int = 0x00000004;
pub const SA_ONSTACK: c_int = 0x08000000;
pub const SA_RESTART: c_int = 0x10000000;
pub const SA_RESETHAND: c_int = 0x80000000u32 as c_int;

pub const SIG_BLOCK: c_int = 0;
pub const SIG_UNBLOCK: c_int = 1;
pub const SIG_SETMASK: c_int = 2;

pub const MINSIGSTKSZ: usize = 2048;
pub const SIGSTKSZ: usize = 8192;
pub const SS_ONSTACK: c_int = 1;
pub const SS_DISABLE: c_int = 2;

// =========================================================================
// Clock constants (from uapi/rust/consts.rs)
// =========================================================================

pub const CLOCK_REALTIME: clockid_t = 1;
pub const CLOCK_MONOTONIC: clockid_t = 0;

// =========================================================================
// Fcntl constants (from uapi/rust/consts.rs)
// =========================================================================

pub const F_DUPFD: c_int = 0;
pub const F_GETFD: c_int = 1;
pub const F_SETFD: c_int = 2;
pub const F_GETFL: c_int = 3;
pub const F_SETFL: c_int = 4;
pub const F_GETLK: c_int = 5;
pub const F_SETLK: c_int = 6;
pub const F_SETLKW: c_int = 7;
pub const F_DUPFD_CLOEXEC: c_int = 1030;
pub const FD_CLOEXEC: c_int = 1;

pub const F_RDLCK: c_int = 0;
pub const F_WRLCK: c_int = 1;
pub const F_UNLCK: c_int = 2;

// =========================================================================
// Process constants
// =========================================================================

pub const WNOHANG: c_int = 1;
pub const WUNTRACED: c_int = 2;

pub const EXIT_SUCCESS: c_int = 0;
pub const EXIT_FAILURE: c_int = 1;

// =========================================================================
// File descriptor constants
// =========================================================================

pub const STDIN_FILENO: c_int = 0;
pub const STDOUT_FILENO: c_int = 1;
pub const STDERR_FILENO: c_int = 2;

// =========================================================================
// Pthread constants
// =========================================================================

pub const PTHREAD_MUTEX_NORMAL: c_int = 0;
pub const PTHREAD_MUTEX_ERRORCHECK: c_int = 2;
pub const PTHREAD_MUTEX_RECURSIVE: c_int = 1;
pub const PTHREAD_MUTEX_DEFAULT: c_int = 0;

pub const PTHREAD_CREATE_JOINABLE: c_int = 0;
pub const PTHREAD_CREATE_DETACHED: c_int = 1;

pub const PTHREAD_STACK_MIN: size_t = 16384;

pub const PTHREAD_PROCESS_PRIVATE: c_int = 0;
pub const PTHREAD_PROCESS_SHARED: c_int = 1;

// =========================================================================
// Sysconf constants
// =========================================================================

pub const _SC_CLK_TCK: c_int = 3;
pub const _SC_OPEN_MAX: c_int = 5;
pub const _SC_PAGESIZE: c_int = 47;
pub const _SC_PAGE_SIZE: c_int = _SC_PAGESIZE;
pub const _SC_NPROCESSORS_CONF: c_int = 57;
pub const _SC_NPROCESSORS_ONLN: c_int = 58;

// =========================================================================
// Resource limit constants
// =========================================================================

pub const RLIMIT_NOFILE: c_int = 7;
pub const RLIMIT_STACK: c_int = 3;
pub const RLIM_INFINITY: rlim_t = u64::MAX;

// =========================================================================
// Getaddrinfo constants
// =========================================================================

pub const AI_PASSIVE: c_int = 1;
pub const AI_CANONNAME: c_int = 2;
pub const AI_NUMERICHOST: c_int = 4;
pub const AI_NUMERICSERV: c_int = 0x400;
pub const AI_ADDRCONFIG: c_int = 0x20;

pub const EAI_AGAIN: c_int = 2;
pub const EAI_BADFLAGS: c_int = 3;
pub const EAI_FAIL: c_int = 4;
pub const EAI_FAMILY: c_int = 5;
pub const EAI_MEMORY: c_int = 6;
pub const EAI_NONAME: c_int = 8;
pub const EAI_SERVICE: c_int = 9;
pub const EAI_SOCKTYPE: c_int = 10;
pub const EAI_SYSTEM: c_int = 11;
pub const EAI_OVERFLOW: c_int = 14;

pub const NI_MAXHOST: usize = 1025;
pub const NI_MAXSERV: usize = 32;
pub const NI_NUMERICHOST: c_int = 1;
pub const NI_NUMERICSERV: c_int = 2;

// =========================================================================
// Termios constants
// =========================================================================

pub const TCSANOW: c_int = 0;
pub const TCSADRAIN: c_int = 1;
pub const TCSAFLUSH: c_int = 2;
pub const NCCS: usize = 32;

// c_iflag
pub const IGNBRK: u32 = 0x00000001;
pub const BRKINT: u32 = 0x00000002;
pub const IGNPAR: u32 = 0x00000004;
pub const PARMRK: u32 = 0x00000008;
pub const INPCK: u32 = 0x00000010;
pub const ISTRIP: u32 = 0x00000020;
pub const INLCR: u32 = 0x00000040;
pub const IGNCR: u32 = 0x00000080;
pub const ICRNL: u32 = 0x00000100;
pub const IXON: u32 = 0x00000400;
pub const IXANY: u32 = 0x00000800;
pub const IXOFF: u32 = 0x00001000;

// c_oflag
pub const OPOST: u32 = 0x00000001;
pub const ONLCR: u32 = 0x00000004;

// c_cflag
pub const CSIZE: u32 = 0x00000030;
pub const CS5: u32 = 0x00000000;
pub const CS6: u32 = 0x00000010;
pub const CS7: u32 = 0x00000020;
pub const CS8: u32 = 0x00000030;
pub const CSTOPB: u32 = 0x00000040;
pub const CREAD: u32 = 0x00000080;
pub const PARENB: u32 = 0x00000100;
pub const PARODD: u32 = 0x00000200;
pub const HUPCL: u32 = 0x00000400;
pub const CLOCAL: u32 = 0x00000800;

// c_lflag
pub const ISIG: u32 = 0x00000001;
pub const ICANON: u32 = 0x00000002;
pub const ECHO: u32 = 0x00000008;
pub const ECHOE: u32 = 0x00000010;
pub const ECHOK: u32 = 0x00000020;
pub const ECHONL: u32 = 0x00000040;
pub const NOFLSH: u32 = 0x00000080;
pub const TOSTOP: u32 = 0x00000100;
pub const IEXTEN: u32 = 0x00008000;

// c_cc indices
pub const VINTR: usize = 0;
pub const VQUIT: usize = 1;
pub const VERASE: usize = 2;
pub const VKILL: usize = 3;
pub const VEOF: usize = 4;
pub const VTIME: usize = 5;
pub const VMIN: usize = 6;
pub const VSTART: usize = 8;
pub const VSTOP: usize = 9;
pub const VSUSP: usize = 10;
pub const VEOL: usize = 11;
pub const VREPRINT: usize = 12;
pub const VWERASE: usize = 14;
pub const VLNEXT: usize = 15;
pub const VEOL2: usize = 16;

// Baud rates
pub const B0: u32 = 0;
pub const B50: u32 = 1;
pub const B75: u32 = 2;
pub const B110: u32 = 3;
pub const B134: u32 = 4;
pub const B150: u32 = 5;
pub const B200: u32 = 6;
pub const B300: u32 = 7;
pub const B600: u32 = 8;
pub const B1200: u32 = 9;
pub const B1800: u32 = 10;
pub const B2400: u32 = 11;
pub const B4800: u32 = 12;
pub const B9600: u32 = 13;
pub const B19200: u32 = 14;
pub const B38400: u32 = 15;
pub const B57600: u32 = 4097;
pub const B115200: u32 = 4098;
pub const B230400: u32 = 4099;

// ioctl requests
pub const TIOCGWINSZ: c_ulong = 0x5413;
pub const TIOCSWINSZ: c_ulong = 0x5414;
pub const TIOCGPGRP: c_ulong = 0x540F;
pub const TIOCSPGRP: c_ulong = 0x5410;
pub const TIOCSCTTY: c_ulong = 0x540E;
pub const TIOCNOTTY: c_ulong = 0x5422;
pub const FIONBIO: c_ulong = 0x5421;
pub const FIONREAD: c_ulong = 0x541B;

// Directory entry types
pub const DT_UNKNOWN: u8 = 0;
pub const DT_FIFO: u8 = 1;
pub const DT_CHR: u8 = 2;
pub const DT_DIR: u8 = 4;
pub const DT_BLK: u8 = 6;
pub const DT_REG: u8 = 8;
pub const DT_LNK: u8 = 10;
pub const DT_SOCK: u8 = 12;

// FD_SETSIZE for select()
pub const FD_SETSIZE: usize = 1024;

// Access mode flags
pub const F_OK: c_int = 0;
pub const R_OK: c_int = 4;
pub const W_OK: c_int = 2;
pub const X_OK: c_int = 1;

// Wait status macros are functions — provide inline helpers
#[inline]
pub fn WIFEXITED(status: c_int) -> bool {
    (status & 0x7f) == 0
}
#[inline]
pub fn WEXITSTATUS(status: c_int) -> c_int {
    (status >> 8) & 0xff
}
#[inline]
pub fn WIFSIGNALED(status: c_int) -> bool {
    (status & 0x7f) != 0 && (status & 0x7f) != 0x7f
}
#[inline]
pub fn WTERMSIG(status: c_int) -> c_int {
    status & 0x7f
}
#[inline]
pub fn WIFSTOPPED(status: c_int) -> bool {
    (status & 0xff) == 0x7f
}
#[inline]
pub fn WSTOPSIG(status: c_int) -> c_int {
    (status >> 8) & 0xff
}

// PATH_MAX
pub const PATH_MAX: usize = 4096;
pub const PIPE_BUF: usize = 4096;
pub const NAME_MAX: usize = 255;
pub const IOV_MAX: usize = 1024;

// =========================================================================
// Struct definitions
// =========================================================================

// timespec — from time_impl.rs: tv_sec: i64, tv_nsec: i64
#[repr(C)]
#[derive(Clone, Copy)]
pub struct timespec {
    pub tv_sec: time_t,
    pub tv_nsec: c_long,
}

// timeval — from time_impl.rs: tv_sec: i64, tv_usec: i64
#[repr(C)]
#[derive(Clone, Copy)]
pub struct timeval {
    pub tv_sec: time_t,
    pub tv_usec: suseconds_t,
}

// stat — from unistd.rs Stat struct (FreeBSD-compatible layout)
// Fields: st_dev(u64), st_ino(u64), st_nlink(u64), st_mode(u16),
//   st_padding0(i16), st_uid(u32), st_gid(u32), st_padding1(i32),
//   st_rdev(u64), st_atim(Timespec), st_mtim(Timespec), st_ctim(Timespec),
//   st_birthtim(Timespec), st_size(i64), st_blocks(i64), st_blksize(i32),
//   st_flags(u32), st_gen(u64), st_spare([u64;10])
#[repr(C)]
#[derive(Clone, Copy)]
pub struct stat {
    pub st_dev: dev_t,
    pub st_ino: ino_t,
    pub st_nlink: nlink_t,
    pub st_mode: u16,
    __st_padding0: i16,
    pub st_uid: uid_t,
    pub st_gid: gid_t,
    __st_padding1: i32,
    pub st_rdev: dev_t,
    pub st_atime: time_t,
    pub st_atime_nsec: c_long,
    pub st_mtime: time_t,
    pub st_mtime_nsec: c_long,
    pub st_ctime: time_t,
    pub st_ctime_nsec: c_long,
    pub st_birthtime: time_t,
    pub st_birthtime_nsec: c_long,
    pub st_size: off_t,
    pub st_blocks: blkcnt_t,
    pub st_blksize: i32,
    pub st_flags: u32,
    pub st_gen: u64,
    __st_spare: [u64; 10],
}

// dirent — from dirent_impl.rs
// Fields: d_ino(u64), d_off(i64), d_reclen(u16), d_type(u8),
//   d_pad0(u8), d_namlen(u16), d_pad1(u16), d_name([u8;256])
#[repr(C)]
pub struct dirent {
    pub d_ino: ino_t,
    pub d_off: i64,
    pub d_reclen: u16,
    pub d_type: u8,
    __d_pad0: u8,
    pub d_namlen: u16,
    __d_pad1: u16,
    pub d_name: [c_char; 256],
}

// sockaddr
#[repr(C)]
#[derive(Clone, Copy)]
pub struct sockaddr {
    pub sa_family: sa_family_t,
    pub sa_data: [c_char; 14],
}

// sockaddr_un — from C header: sun_family(u16), sun_path([char;108])
#[repr(C)]
#[derive(Clone, Copy)]
pub struct sockaddr_un {
    pub sun_family: sa_family_t,
    pub sun_path: [c_char; 108],
}

// in_addr
#[repr(C)]
#[derive(Clone, Copy)]
pub struct in_addr {
    pub s_addr: in_addr_t,
}

// sockaddr_in
#[repr(C)]
#[derive(Clone, Copy)]
pub struct sockaddr_in {
    pub sin_family: sa_family_t,
    pub sin_port: in_port_t,
    pub sin_addr: in_addr,
    pub sin_zero: [u8; 8],
}

// sockaddr_in6 (stub for compatibility)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct in6_addr {
    pub s6_addr: [u8; 16],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct sockaddr_in6 {
    pub sin6_family: sa_family_t,
    pub sin6_port: in_port_t,
    pub sin6_flowinfo: u32,
    pub sin6_addr: in6_addr,
    pub sin6_scope_id: u32,
}

// sockaddr_storage
#[repr(C)]
#[derive(Clone, Copy)]
pub struct sockaddr_storage {
    pub ss_family: sa_family_t,
    __ss_padding: [u8; 118],
    __ss_align: u64,
}

// pollfd — from types.rs PollFd: fd(i32), events(i16), revents(i16)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct pollfd {
    pub fd: c_int,
    pub events: c_short,
    pub revents: c_short,
}

// epoll_event — packed on x86_64
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct epoll_event {
    pub events: u32,
    pub u64: u64,
}

// addrinfo
#[repr(C)]
pub struct addrinfo {
    pub ai_flags: c_int,
    pub ai_family: c_int,
    pub ai_socktype: c_int,
    pub ai_protocol: c_int,
    pub ai_addrlen: socklen_t,
    pub ai_addr: *mut sockaddr,
    pub ai_canonname: *mut c_char,
    pub ai_next: *mut addrinfo,
}

// sigset_t — from signal_impl.rs Sigset: bits(u32)
// Represented as a u32 bitmask
#[repr(C)]
#[derive(Clone, Copy)]
pub struct sigset_t {
    pub __val: [c_ulong; 1],
}

// sigaction — from signal_impl.rs: sa_handler(usize), sa_mask(Sigset), sa_flags(i32), sa_restorer(usize)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct sigaction {
    pub sa_sigaction: sighandler_t,
    pub sa_mask: sigset_t,
    pub sa_flags: c_int,
    pub sa_restorer: Option<unsafe extern "C" fn()>,
}

// siginfo_t (simplified)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct siginfo_t {
    pub si_signo: c_int,
    pub si_code: c_int,
    pub si_errno: c_int,
    pub si_pid: pid_t,
    pub si_uid: uid_t,
    pub si_status: c_int,
    pub si_addr: *mut c_void,
    _pad: [c_long; 4],
}

// stack_t for sigaltstack
#[repr(C)]
#[derive(Clone, Copy)]
pub struct stack_t {
    pub ss_sp: *mut c_void,
    pub ss_flags: c_int,
    pub ss_size: size_t,
}

// pthread opaque types — sizes from pthread_impl.rs
// PthreadAttrT: stack_size(u64) + detach_state(u32) + _pad(u32) = 16 bytes
#[repr(C)]
pub struct pthread_attr_t {
    __private: [u8; 16],
}

// PthreadMutexT: 32 bytes (24 bytes mutex data + kind byte + padding)
#[repr(C, align(8))]
pub struct pthread_mutex_t {
    __private: [u8; 32],
}

// PthreadCondT: 8 bytes
#[repr(C)]
pub struct pthread_cond_t {
    __private: [u8; 8],
}

// PthreadRwlockT: 16 bytes
#[repr(C)]
pub struct pthread_rwlock_t {
    __private: [u8; 16],
}

// PthreadOnceT: 8 bytes
#[repr(C)]
pub struct pthread_once_t {
    __private: [u8; 8],
}

pub const PTHREAD_ONCE_INIT: pthread_once_t = pthread_once_t { __private: [0; 8] };
pub const PTHREAD_MUTEX_INITIALIZER: pthread_mutex_t = pthread_mutex_t { __private: [0; 32] };
pub const PTHREAD_COND_INITIALIZER: pthread_cond_t = pthread_cond_t { __private: [0; 8] };
pub const PTHREAD_RWLOCK_INITIALIZER: pthread_rwlock_t = pthread_rwlock_t { __private: [0; 16] };

// PthreadMutexattrT: kind(i32) = 4 bytes
#[repr(C)]
pub struct pthread_mutexattr_t {
    __private: [u8; 4],
}

// PthreadCondattrT: 4 bytes
#[repr(C)]
pub struct pthread_condattr_t {
    __private: [u8; 4],
}

// PthreadRwlockattrT: 4 bytes
#[repr(C)]
pub struct pthread_rwlockattr_t {
    __private: [u8; 4],
}

// PthreadBarrierT: 16 bytes
#[repr(C)]
pub struct pthread_barrier_t {
    __private: [u8; 16],
}

// PthreadBarrierattrT: 4 bytes
#[repr(C)]
pub struct pthread_barrierattr_t {
    __private: [u8; 4],
}

// rlimit
#[repr(C)]
#[derive(Clone, Copy)]
pub struct rlimit {
    pub rlim_cur: rlim_t,
    pub rlim_max: rlim_t,
}

// dl_phdr_info for dl_iterate_phdr
#[repr(C)]
pub struct dl_phdr_info {
    pub dlpi_addr: u64,
    pub dlpi_name: *const c_char,
    pub dlpi_phdr: *const c_void,
    pub dlpi_phnum: u16,
}

// iovec — from unistd.rs Iovec
#[repr(C)]
#[derive(Clone, Copy)]
pub struct iovec {
    pub iov_base: *mut c_void,
    pub iov_len: size_t,
}

// msghdr
#[repr(C)]
pub struct msghdr {
    pub msg_name: *mut c_void,
    pub msg_namelen: socklen_t,
    pub msg_iov: *mut iovec,
    pub msg_iovlen: size_t,
    pub msg_control: *mut c_void,
    pub msg_controllen: size_t,
    pub msg_flags: c_int,
}

// cmsghdr
#[repr(C)]
pub struct cmsghdr {
    pub cmsg_len: size_t,
    pub cmsg_level: c_int,
    pub cmsg_type: c_int,
}

// fd_set — from select_impl.rs: fds_bits([u64; 16])
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fd_set {
    pub fds_bits: [c_ulong; FD_SETSIZE / 64],
}

// termios — from types.rs Termios
#[repr(C)]
#[derive(Clone, Copy)]
pub struct termios {
    pub c_iflag: u32,
    pub c_oflag: u32,
    pub c_cflag: u32,
    pub c_lflag: u32,
    pub c_line: u8,
    pub c_cc: [u8; NCCS],
    pub c_ispeed: u32,
    pub c_ospeed: u32,
}

// winsize
#[repr(C)]
#[derive(Clone, Copy)]
pub struct winsize {
    pub ws_row: c_ushort,
    pub ws_col: c_ushort,
    pub ws_xpixel: c_ushort,
    pub ws_ypixel: c_ushort,
}

// linger
#[repr(C)]
#[derive(Clone, Copy)]
pub struct linger {
    pub l_onoff: c_int,
    pub l_linger: c_int,
}

// ip_mreq
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ip_mreq {
    pub imr_multiaddr: in_addr,
    pub imr_interface: in_addr,
}

// rusage (stub)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct rusage {
    pub ru_utime: timeval,
    pub ru_stime: timeval,
    pub ru_maxrss: c_long,
    pub ru_ixrss: c_long,
    pub ru_idrss: c_long,
    pub ru_isrss: c_long,
    pub ru_minflt: c_long,
    pub ru_majflt: c_long,
    pub ru_nswap: c_long,
    pub ru_inblock: c_long,
    pub ru_oublock: c_long,
    pub ru_msgsnd: c_long,
    pub ru_msgrcv: c_long,
    pub ru_nsignals: c_long,
    pub ru_nvcsw: c_long,
    pub ru_nivcsw: c_long,
}

// flock
#[repr(C)]
#[derive(Clone, Copy)]
pub struct flock {
    pub l_type: c_short,
    pub l_whence: c_short,
    pub l_start: off_t,
    pub l_len: off_t,
    pub l_pid: pid_t,
}

// passwd
#[repr(C)]
pub struct passwd {
    pub pw_name: *mut c_char,
    pub pw_passwd: *mut c_char,
    pub pw_uid: uid_t,
    pub pw_gid: gid_t,
    pub pw_gecos: *mut c_char,
    pub pw_dir: *mut c_char,
    pub pw_shell: *mut c_char,
}

// group
#[repr(C)]
pub struct group {
    pub gr_name: *mut c_char,
    pub gr_passwd: *mut c_char,
    pub gr_gid: gid_t,
    pub gr_mem: *mut *mut c_char,
}

// utsname
#[repr(C)]
pub struct utsname {
    pub sysname: [c_char; 65],
    pub nodename: [c_char; 65],
    pub release: [c_char; 65],
    pub version: [c_char; 65],
    pub machine: [c_char; 65],
    pub domainname: [c_char; 65],
}

// tm (broken-down time) — from time_impl.rs Tm
#[repr(C)]
#[derive(Clone, Copy)]
pub struct tm {
    pub tm_sec: c_int,
    pub tm_min: c_int,
    pub tm_hour: c_int,
    pub tm_mday: c_int,
    pub tm_mon: c_int,
    pub tm_year: c_int,
    pub tm_wday: c_int,
    pub tm_yday: c_int,
    pub tm_isdst: c_int,
}

// Elf64 types needed by dl_iterate_phdr consumers
pub type Elf64_Half = u16;
pub type Elf64_Word = u32;
pub type Elf64_Xword = u64;
pub type Elf64_Addr = u64;
pub type Elf64_Off = u64;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Elf64_Phdr {
    pub p_type: Elf64_Word,
    pub p_flags: Elf64_Word,
    pub p_offset: Elf64_Off,
    pub p_vaddr: Elf64_Addr,
    pub p_paddr: Elf64_Addr,
    pub p_filesz: Elf64_Xword,
    pub p_memsz: Elf64_Xword,
    pub p_align: Elf64_Xword,
}

// posix_spawn types (opaque)
pub enum posix_spawn_file_actions_t {}
pub enum posix_spawnattr_t {}

// =========================================================================
// Extern "C" function declarations
// =========================================================================

unsafe extern "C" {
    // --- errno ---
    pub fn __errno_location() -> *mut c_int;

    // --- Memory ---
    pub fn malloc(size: size_t) -> *mut c_void;
    pub fn calloc(nobj: size_t, size: size_t) -> *mut c_void;
    pub fn realloc(ptr: *mut c_void, size: size_t) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
    pub fn posix_memalign(memptr: *mut *mut c_void, align: size_t, size: size_t) -> c_int;
    pub fn aligned_alloc(align: size_t, size: size_t) -> *mut c_void;
    pub fn memalign(align: size_t, size: size_t) -> *mut c_void;

    // --- String ---
    pub fn strlen(s: *const c_char) -> size_t;
    pub fn strnlen(s: *const c_char, maxlen: size_t) -> size_t;
    pub fn strcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char;
    pub fn strncpy(dst: *mut c_char, src: *const c_char, n: size_t) -> *mut c_char;
    pub fn strcat(dst: *mut c_char, src: *const c_char) -> *mut c_char;
    pub fn strncat(dst: *mut c_char, src: *const c_char, n: size_t) -> *mut c_char;
    pub fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    pub fn strncmp(s1: *const c_char, s2: *const c_char, n: size_t) -> c_int;
    pub fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    pub fn strrchr(s: *const c_char, c: c_int) -> *mut c_char;
    pub fn strstr(haystack: *const c_char, needle: *const c_char) -> *mut c_char;
    pub fn strtok_r(
        s: *mut c_char,
        delim: *const c_char,
        saveptr: *mut *mut c_char,
    ) -> *mut c_char;
    pub fn strtol(s: *const c_char, endp: *mut *mut c_char, base: c_int) -> c_long;
    pub fn strtoul(s: *const c_char, endp: *mut *mut c_char, base: c_int) -> c_ulong;
    pub fn strtoll(s: *const c_char, endp: *mut *mut c_char, base: c_int) -> c_longlong;
    pub fn strtoull(s: *const c_char, endp: *mut *mut c_char, base: c_int) -> c_ulonglong;
    pub fn strtof(s: *const c_char, endp: *mut *mut c_char) -> c_float;
    pub fn strtod(s: *const c_char, endp: *mut *mut c_char) -> c_double;
    pub fn memcpy(dst: *mut c_void, src: *const c_void, n: size_t) -> *mut c_void;
    pub fn memmove(dst: *mut c_void, src: *const c_void, n: size_t) -> *mut c_void;
    pub fn memset(s: *mut c_void, c: c_int, n: size_t) -> *mut c_void;
    pub fn memcmp(s1: *const c_void, s2: *const c_void, n: size_t) -> c_int;
    pub fn memchr(s: *const c_void, c: c_int, n: size_t) -> *mut c_void;
    pub fn strdup(s: *const c_char) -> *mut c_char;
    pub fn strndup(s: *const c_char, n: size_t) -> *mut c_char;
    pub fn strerror(errnum: c_int) -> *mut c_char;
    pub fn strerror_r(errnum: c_int, buf: *mut c_char, buflen: size_t) -> c_int;
    pub fn strsignal(sig: c_int) -> *mut c_char;
    pub fn strcasecmp(s1: *const c_char, s2: *const c_char) -> c_int;
    pub fn strncasecmp(s1: *const c_char, s2: *const c_char, n: size_t) -> c_int;
    pub fn stpcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char;
    pub fn stpncpy(dst: *mut c_char, src: *const c_char, n: size_t) -> *mut c_char;
    pub fn strspn(s: *const c_char, accept: *const c_char) -> size_t;
    pub fn strcspn(s: *const c_char, reject: *const c_char) -> size_t;
    pub fn strpbrk(s: *const c_char, accept: *const c_char) -> *mut c_char;
    pub fn strtoimax(s: *const c_char, endp: *mut *mut c_char, base: c_int) -> isize;
    pub fn strtoumax(s: *const c_char, endp: *mut *mut c_char, base: c_int) -> usize;
    pub fn memrchr(s: *const c_void, c: c_int, n: size_t) -> *mut c_void;
    pub fn explicit_bzero(s: *mut c_void, n: size_t);
    pub fn bcmp(s1: *const c_void, s2: *const c_void, n: size_t) -> c_int;

    // --- I/O ---
    pub fn open(path: *const c_char, oflag: c_int, ...) -> c_int;
    pub fn openat(dirfd: c_int, path: *const c_char, oflag: c_int, ...) -> c_int;
    pub fn close(fd: c_int) -> c_int;
    pub fn read(fd: c_int, buf: *mut c_void, count: size_t) -> ssize_t;
    pub fn write(fd: c_int, buf: *const c_void, count: size_t) -> ssize_t;
    pub fn pread(fd: c_int, buf: *mut c_void, count: size_t, offset: off_t) -> ssize_t;
    pub fn pwrite(fd: c_int, buf: *const c_void, count: size_t, offset: off_t) -> ssize_t;
    pub fn readv(fd: c_int, iov: *const iovec, iovcnt: c_int) -> ssize_t;
    pub fn writev(fd: c_int, iov: *const iovec, iovcnt: c_int) -> ssize_t;
    pub fn lseek(fd: c_int, offset: off_t, whence: c_int) -> off_t;
    pub fn dup(oldfd: c_int) -> c_int;
    pub fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    pub fn dup3(oldfd: c_int, newfd: c_int, flags: c_int) -> c_int;
    pub fn pipe(fds: *mut c_int) -> c_int;
    pub fn pipe2(fds: *mut c_int, flags: c_int) -> c_int;
    pub fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
    pub fn stat(path: *const c_char, buf: *mut stat) -> c_int;
    pub fn fstat(fd: c_int, buf: *mut stat) -> c_int;
    pub fn lstat(path: *const c_char, buf: *mut stat) -> c_int;
    pub fn fstatat(dirfd: c_int, path: *const c_char, buf: *mut stat, flag: c_int) -> c_int;
    pub fn unlink(path: *const c_char) -> c_int;
    pub fn unlinkat(dirfd: c_int, path: *const c_char, flag: c_int) -> c_int;
    pub fn rmdir(path: *const c_char) -> c_int;
    pub fn mkdir(path: *const c_char, mode: mode_t) -> c_int;
    pub fn mkdirat(dirfd: c_int, path: *const c_char, mode: mode_t) -> c_int;
    pub fn rename(old: *const c_char, new: *const c_char) -> c_int;
    pub fn renameat(
        olddirfd: c_int,
        old: *const c_char,
        newdirfd: c_int,
        new: *const c_char,
    ) -> c_int;
    pub fn access(path: *const c_char, amode: c_int) -> c_int;
    pub fn faccessat(dirfd: c_int, path: *const c_char, amode: c_int, flag: c_int) -> c_int;
    pub fn chdir(path: *const c_char) -> c_int;
    pub fn fchdir(fd: c_int) -> c_int;
    pub fn getcwd(buf: *mut c_char, size: size_t) -> *mut c_char;
    pub fn symlink(target: *const c_char, linkpath: *const c_char) -> c_int;
    pub fn symlinkat(target: *const c_char, newdirfd: c_int, linkpath: *const c_char) -> c_int;
    pub fn readlink(path: *const c_char, buf: *mut c_char, bufsiz: size_t) -> ssize_t;
    pub fn readlinkat(
        dirfd: c_int,
        path: *const c_char,
        buf: *mut c_char,
        bufsiz: size_t,
    ) -> ssize_t;
    pub fn link(old: *const c_char, new: *const c_char) -> c_int;
    pub fn linkat(
        olddirfd: c_int,
        old: *const c_char,
        newdirfd: c_int,
        new: *const c_char,
        flags: c_int,
    ) -> c_int;
    pub fn opendir(name: *const c_char) -> *mut DIR;
    pub fn readdir(dirp: *mut DIR) -> *mut dirent;
    pub fn closedir(dirp: *mut DIR) -> c_int;
    pub fn dirfd(dirp: *mut DIR) -> c_int;
    pub fn rewinddir(dirp: *mut DIR);
    pub fn isatty(fd: c_int) -> c_int;
    pub fn ttyname(fd: c_int) -> *mut c_char;
    pub fn ttyname_r(fd: c_int, buf: *mut c_char, buflen: size_t) -> c_int;
    pub fn ftruncate(fd: c_int, length: off_t) -> c_int;
    pub fn truncate(path: *const c_char, length: off_t) -> c_int;
    pub fn chmod(path: *const c_char, mode: mode_t) -> c_int;
    pub fn fchmod(fd: c_int, mode: mode_t) -> c_int;
    pub fn fchmodat(dirfd: c_int, path: *const c_char, mode: mode_t, flag: c_int) -> c_int;
    pub fn chown(path: *const c_char, owner: uid_t, group: gid_t) -> c_int;
    pub fn fchown(fd: c_int, owner: uid_t, group: gid_t) -> c_int;
    pub fn lchown(path: *const c_char, owner: uid_t, group: gid_t) -> c_int;
    pub fn fchownat(
        dirfd: c_int,
        path: *const c_char,
        owner: uid_t,
        group: gid_t,
        flag: c_int,
    ) -> c_int;
    pub fn umask(mask: mode_t) -> mode_t;
    pub fn mkfifo(path: *const c_char, mode: mode_t) -> c_int;
    pub fn realpath(path: *const c_char, resolved_path: *mut c_char) -> *mut c_char;
    pub fn utimensat(
        dirfd: c_int,
        path: *const c_char,
        times: *const timespec,
        flag: c_int,
    ) -> c_int;
    pub fn futimens(fd: c_int, times: *const timespec) -> c_int;
    pub fn pathconf(path: *const c_char, name: c_int) -> c_long;
    pub fn fpathconf(fd: c_int, name: c_int) -> c_long;
    pub fn confstr(name: c_int, buf: *mut c_char, len: size_t) -> size_t;
    pub fn flock(fd: c_int, operation: c_int) -> c_int;

    // --- Mmap ---
    pub fn mmap(
        addr: *mut c_void,
        length: size_t,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        offset: off_t,
    ) -> *mut c_void;
    pub fn munmap(addr: *mut c_void, length: size_t) -> c_int;
    pub fn mprotect(addr: *mut c_void, length: size_t, prot: c_int) -> c_int;
    pub fn madvise(addr: *mut c_void, length: size_t, advice: c_int) -> c_int;

    // --- Process ---
    pub fn fork() -> pid_t;
    pub fn execve(
        path: *const c_char,
        argv: *const *const c_char,
        envp: *const *const c_char,
    ) -> c_int;
    pub fn execvp(file: *const c_char, argv: *const *const c_char) -> c_int;
    pub fn _exit(status: c_int) -> !;
    pub fn getpid() -> pid_t;
    pub fn getppid() -> pid_t;
    pub fn kill(pid: pid_t, sig: c_int) -> c_int;
    pub fn waitpid(pid: pid_t, status: *mut c_int, options: c_int) -> pid_t;
    pub fn abort() -> !;
    pub fn exit(status: c_int) -> !;
    pub fn _Exit(status: c_int) -> !;
    pub fn setsid() -> pid_t;
    pub fn setpgid(pid: pid_t, pgid: pid_t) -> c_int;
    pub fn getpgid(pid: pid_t) -> pid_t;
    pub fn getpgrp() -> pid_t;
    pub fn getsid(pid: pid_t) -> pid_t;
    pub fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;

    // --- Env ---
    pub fn getenv(name: *const c_char) -> *mut c_char;
    pub fn setenv(name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;
    pub fn unsetenv(name: *const c_char) -> c_int;
    pub fn putenv(string: *mut c_char) -> c_int;

    // --- Socket ---
    pub fn socket(domain: c_int, ty: c_int, protocol: c_int) -> c_int;
    pub fn bind(sockfd: c_int, addr: *const sockaddr, addrlen: socklen_t) -> c_int;
    pub fn listen(sockfd: c_int, backlog: c_int) -> c_int;
    pub fn accept(sockfd: c_int, addr: *mut sockaddr, addrlen: *mut socklen_t) -> c_int;
    pub fn connect(sockfd: c_int, addr: *const sockaddr, addrlen: socklen_t) -> c_int;
    pub fn shutdown(sockfd: c_int, how: c_int) -> c_int;
    pub fn socketpair(domain: c_int, ty: c_int, protocol: c_int, sv: *mut c_int) -> c_int;
    pub fn send(sockfd: c_int, buf: *const c_void, len: size_t, flags: c_int) -> ssize_t;
    pub fn recv(sockfd: c_int, buf: *mut c_void, len: size_t, flags: c_int) -> ssize_t;
    pub fn sendto(
        sockfd: c_int,
        buf: *const c_void,
        len: size_t,
        flags: c_int,
        dest_addr: *const sockaddr,
        addrlen: socklen_t,
    ) -> ssize_t;
    pub fn recvfrom(
        sockfd: c_int,
        buf: *mut c_void,
        len: size_t,
        flags: c_int,
        src_addr: *mut sockaddr,
        addrlen: *mut socklen_t,
    ) -> ssize_t;
    pub fn sendmsg(sockfd: c_int, msg: *const msghdr, flags: c_int) -> ssize_t;
    pub fn recvmsg(sockfd: c_int, msg: *mut msghdr, flags: c_int) -> ssize_t;
    pub fn setsockopt(
        sockfd: c_int,
        level: c_int,
        optname: c_int,
        optval: *const c_void,
        optlen: socklen_t,
    ) -> c_int;
    pub fn getsockopt(
        sockfd: c_int,
        level: c_int,
        optname: c_int,
        optval: *mut c_void,
        optlen: *mut socklen_t,
    ) -> c_int;
    pub fn getsockname(sockfd: c_int, addr: *mut sockaddr, addrlen: *mut socklen_t) -> c_int;
    pub fn getpeername(sockfd: c_int, addr: *mut sockaddr, addrlen: *mut socklen_t) -> c_int;
    pub fn getaddrinfo(
        node: *const c_char,
        service: *const c_char,
        hints: *const addrinfo,
        res: *mut *mut addrinfo,
    ) -> c_int;
    pub fn freeaddrinfo(res: *mut addrinfo);
    pub fn gai_strerror(ecode: c_int) -> *const c_char;
    pub fn getnameinfo(
        sa: *const sockaddr,
        salen: socklen_t,
        host: *mut c_char,
        hostlen: socklen_t,
        serv: *mut c_char,
        servlen: socklen_t,
        flags: c_int,
    ) -> c_int;
    pub fn poll(fds: *mut pollfd, nfds: nfds_t, timeout: c_int) -> c_int;
    pub fn select(
        nfds: c_int,
        readfds: *mut fd_set,
        writefds: *mut fd_set,
        exceptfds: *mut fd_set,
        timeout: *mut timeval,
    ) -> c_int;
    pub fn epoll_create1(flags: c_int) -> c_int;
    pub fn epoll_ctl(epfd: c_int, op: c_int, fd: c_int, event: *mut epoll_event) -> c_int;
    pub fn epoll_wait(
        epfd: c_int,
        events: *mut epoll_event,
        maxevents: c_int,
        timeout: c_int,
    ) -> c_int;
    pub fn inet_ntop(
        af: c_int,
        src: *const c_void,
        dst: *mut c_char,
        size: socklen_t,
    ) -> *const c_char;
    pub fn inet_pton(af: c_int, src: *const c_char, dst: *mut c_void) -> c_int;
    pub fn htons(hostshort: u16) -> u16;
    pub fn ntohs(netshort: u16) -> u16;
    pub fn htonl(hostlong: u32) -> u32;
    pub fn ntohl(netlong: u32) -> u32;
    pub fn inet_addr(cp: *const c_char) -> in_addr_t;

    // --- Pthread ---
    pub fn pthread_create(
        thread: *mut pthread_t,
        attr: *const pthread_attr_t,
        start_routine: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
        arg: *mut c_void,
    ) -> c_int;
    pub fn pthread_join(thread: pthread_t, retval: *mut *mut c_void) -> c_int;
    pub fn pthread_exit(retval: *mut c_void) -> !;
    pub fn pthread_self() -> pthread_t;
    pub fn pthread_detach(thread: pthread_t) -> c_int;
    pub fn pthread_cancel(thread: pthread_t) -> c_int;
    pub fn pthread_equal(t1: pthread_t, t2: pthread_t) -> c_int;

    pub fn pthread_mutex_init(
        mutex: *mut pthread_mutex_t,
        attr: *const pthread_mutexattr_t,
    ) -> c_int;
    pub fn pthread_mutex_lock(mutex: *mut pthread_mutex_t) -> c_int;
    pub fn pthread_mutex_trylock(mutex: *mut pthread_mutex_t) -> c_int;
    pub fn pthread_mutex_unlock(mutex: *mut pthread_mutex_t) -> c_int;
    pub fn pthread_mutex_destroy(mutex: *mut pthread_mutex_t) -> c_int;
    pub fn pthread_mutex_timedlock(
        mutex: *mut pthread_mutex_t,
        abstime: *const timespec,
    ) -> c_int;

    pub fn pthread_mutexattr_init(attr: *mut pthread_mutexattr_t) -> c_int;
    pub fn pthread_mutexattr_destroy(attr: *mut pthread_mutexattr_t) -> c_int;
    pub fn pthread_mutexattr_settype(attr: *mut pthread_mutexattr_t, kind: c_int) -> c_int;
    pub fn pthread_mutexattr_gettype(attr: *const pthread_mutexattr_t, kind: *mut c_int) -> c_int;

    pub fn pthread_cond_init(cond: *mut pthread_cond_t, attr: *const pthread_condattr_t) -> c_int;
    pub fn pthread_cond_signal(cond: *mut pthread_cond_t) -> c_int;
    pub fn pthread_cond_broadcast(cond: *mut pthread_cond_t) -> c_int;
    pub fn pthread_cond_wait(cond: *mut pthread_cond_t, mutex: *mut pthread_mutex_t) -> c_int;
    pub fn pthread_cond_timedwait(
        cond: *mut pthread_cond_t,
        mutex: *mut pthread_mutex_t,
        abstime: *const timespec,
    ) -> c_int;
    pub fn pthread_cond_destroy(cond: *mut pthread_cond_t) -> c_int;

    pub fn pthread_condattr_init(attr: *mut pthread_condattr_t) -> c_int;
    pub fn pthread_condattr_destroy(attr: *mut pthread_condattr_t) -> c_int;

    pub fn pthread_rwlock_init(
        rwlock: *mut pthread_rwlock_t,
        attr: *const pthread_rwlockattr_t,
    ) -> c_int;
    pub fn pthread_rwlock_rdlock(rwlock: *mut pthread_rwlock_t) -> c_int;
    pub fn pthread_rwlock_wrlock(rwlock: *mut pthread_rwlock_t) -> c_int;
    pub fn pthread_rwlock_tryrdlock(rwlock: *mut pthread_rwlock_t) -> c_int;
    pub fn pthread_rwlock_trywrlock(rwlock: *mut pthread_rwlock_t) -> c_int;
    pub fn pthread_rwlock_unlock(rwlock: *mut pthread_rwlock_t) -> c_int;
    pub fn pthread_rwlock_destroy(rwlock: *mut pthread_rwlock_t) -> c_int;

    pub fn pthread_key_create(key: *mut pthread_key_t, dtor: Option<unsafe extern "C" fn(*mut c_void)>) -> c_int;
    pub fn pthread_key_delete(key: pthread_key_t) -> c_int;
    pub fn pthread_setspecific(key: pthread_key_t, value: *const c_void) -> c_int;
    pub fn pthread_getspecific(key: pthread_key_t) -> *mut c_void;

    pub fn pthread_once(
        once_control: *mut pthread_once_t,
        init_routine: unsafe extern "C" fn(),
    ) -> c_int;

    pub fn pthread_attr_init(attr: *mut pthread_attr_t) -> c_int;
    pub fn pthread_attr_destroy(attr: *mut pthread_attr_t) -> c_int;
    pub fn pthread_attr_setstacksize(attr: *mut pthread_attr_t, stacksize: size_t) -> c_int;
    pub fn pthread_attr_getstacksize(attr: *const pthread_attr_t, stacksize: *mut size_t) -> c_int;
    pub fn pthread_attr_setdetachstate(attr: *mut pthread_attr_t, detachstate: c_int) -> c_int;
    pub fn pthread_attr_getdetachstate(
        attr: *const pthread_attr_t,
        detachstate: *mut c_int,
    ) -> c_int;

    pub fn pthread_sigmask(
        how: c_int,
        set: *const sigset_t,
        oldset: *mut sigset_t,
    ) -> c_int;

    pub fn pthread_setcancelstate(state: c_int, oldstate: *mut c_int) -> c_int;
    pub fn pthread_setcanceltype(ctype: c_int, oldtype: *mut c_int) -> c_int;
    pub fn pthread_testcancel();

    pub fn pthread_barrier_init(
        barrier: *mut pthread_barrier_t,
        attr: *const pthread_barrierattr_t,
        count: c_uint,
    ) -> c_int;
    pub fn pthread_barrier_wait(barrier: *mut pthread_barrier_t) -> c_int;
    pub fn pthread_barrier_destroy(barrier: *mut pthread_barrier_t) -> c_int;

    // --- Signal ---
    pub fn signal(signum: c_int, handler: sighandler_t) -> sighandler_t;
    pub fn sigaction(
        signum: c_int,
        act: *const sigaction,
        oldact: *mut sigaction,
    ) -> c_int;
    pub fn sigprocmask(how: c_int, set: *const sigset_t, oldset: *mut sigset_t) -> c_int;
    pub fn sigsuspend(mask: *const sigset_t) -> c_int;
    pub fn sigpending(set: *mut sigset_t) -> c_int;
    pub fn sigemptyset(set: *mut sigset_t) -> c_int;
    pub fn sigfillset(set: *mut sigset_t) -> c_int;
    pub fn sigaddset(set: *mut sigset_t, signum: c_int) -> c_int;
    pub fn sigdelset(set: *mut sigset_t, signum: c_int) -> c_int;
    pub fn sigismember(set: *const sigset_t, signum: c_int) -> c_int;
    pub fn raise(sig: c_int) -> c_int;
    pub fn sigaltstack(ss: *const stack_t, old_ss: *mut stack_t) -> c_int;

    // --- Time ---
    pub fn time(t: *mut time_t) -> time_t;
    pub fn gettimeofday(tv: *mut timeval, tz: *mut c_void) -> c_int;
    pub fn clock_gettime(clock_id: clockid_t, tp: *mut timespec) -> c_int;
    pub fn clock_getres(clock_id: clockid_t, tp: *mut timespec) -> c_int;
    pub fn nanosleep(req: *const timespec, rem: *mut timespec) -> c_int;
    pub fn sleep(seconds: c_uint) -> c_uint;
    pub fn usleep(usec: c_uint) -> c_int;
    pub fn strftime(
        buf: *mut c_char,
        maxsize: size_t,
        format: *const c_char,
        tm: *const tm,
    ) -> size_t;
    pub fn gmtime_r(timer: *const time_t, result: *mut tm) -> *mut tm;
    pub fn gmtime(timer: *const time_t) -> *mut tm;
    pub fn localtime_r(timer: *const time_t, result: *mut tm) -> *mut tm;
    pub fn localtime(timer: *const time_t) -> *mut tm;
    pub fn mktime(tm: *mut tm) -> time_t;
    pub fn asctime_r(tm: *const tm, buf: *mut c_char) -> *mut c_char;
    pub fn asctime(tm: *const tm) -> *mut c_char;
    pub fn ctime_r(timer: *const time_t, buf: *mut c_char) -> *mut c_char;
    pub fn ctime(timer: *const time_t) -> *mut c_char;
    pub fn difftime(time1: time_t, time0: time_t) -> c_double;
    pub fn timegm(tm: *mut tm) -> time_t;
    pub fn strptime(buf: *const c_char, fmt: *const c_char, tm: *mut tm) -> *mut c_char;
    pub fn clock() -> c_long;

    // --- Misc ---
    pub fn sysconf(name: c_int) -> c_long;
    pub fn getrlimit(resource: c_int, rlim: *mut rlimit) -> c_int;
    pub fn setrlimit(resource: c_int, rlim: *const rlimit) -> c_int;
    pub fn dl_iterate_phdr(
        callback: unsafe extern "C" fn(
            info: *mut dl_phdr_info,
            size: size_t,
            data: *mut c_void,
        ) -> c_int,
        data: *mut c_void,
    ) -> c_int;
    pub fn atoi(s: *const c_char) -> c_int;
    pub fn atol(s: *const c_char) -> c_long;
    pub fn atoll(s: *const c_char) -> c_longlong;
    pub fn atof(s: *const c_char) -> c_double;
    pub fn rand() -> c_int;
    pub fn srand(seed: c_uint);
    pub fn abs(i: c_int) -> c_int;
    pub fn labs(i: c_long) -> c_long;
    pub fn llabs(i: c_longlong) -> c_longlong;
    pub fn qsort(
        base: *mut c_void,
        nmemb: size_t,
        size: size_t,
        compar: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int>,
    );
    pub fn bsearch(
        key: *const c_void,
        base: *const c_void,
        nmemb: size_t,
        size: size_t,
        compar: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int>,
    ) -> *mut c_void;
    pub fn getuid() -> uid_t;
    pub fn geteuid() -> uid_t;
    pub fn getgid() -> gid_t;
    pub fn getegid() -> gid_t;
    pub fn getgroups(gidsetsize: c_int, grouplist: *mut gid_t) -> c_int;
    pub fn getpwuid(uid: uid_t) -> *mut passwd;
    pub fn getpwnam(name: *const c_char) -> *mut passwd;
    pub fn getpwuid_r(
        uid: uid_t,
        pwd: *mut passwd,
        buf: *mut c_char,
        buflen: size_t,
        result: *mut *mut passwd,
    ) -> c_int;
    pub fn getpwnam_r(
        name: *const c_char,
        pwd: *mut passwd,
        buf: *mut c_char,
        buflen: size_t,
        result: *mut *mut passwd,
    ) -> c_int;
    pub fn getrandom(buf: *mut c_void, buflen: size_t, flags: c_uint) -> ssize_t;
    pub fn uname(buf: *mut utsname) -> c_int;

    // --- stdio ---
    pub fn printf(format: *const c_char, ...) -> c_int;
    pub fn fprintf(stream: *mut FILE, format: *const c_char, ...) -> c_int;
    pub fn sprintf(buf: *mut c_char, format: *const c_char, ...) -> c_int;
    pub fn snprintf(buf: *mut c_char, size: size_t, format: *const c_char, ...) -> c_int;
    pub fn perror(s: *const c_char);
    pub fn puts(s: *const c_char) -> c_int;
    pub fn fputs(s: *const c_char, stream: *mut FILE) -> c_int;
    pub fn fgets(buf: *mut c_char, n: c_int, stream: *mut FILE) -> *mut c_char;
    pub fn fputc(c: c_int, stream: *mut FILE) -> c_int;
    pub fn fgetc(stream: *mut FILE) -> c_int;
    pub fn getc(stream: *mut FILE) -> c_int;
    pub fn putc(c: c_int, stream: *mut FILE) -> c_int;
    pub fn putchar(c: c_int) -> c_int;
    pub fn getchar() -> c_int;
    pub fn ungetc(c: c_int, stream: *mut FILE) -> c_int;
    pub fn fflush(stream: *mut FILE) -> c_int;
    pub fn feof(stream: *mut FILE) -> c_int;
    pub fn ferror(stream: *mut FILE) -> c_int;
    pub fn clearerr(stream: *mut FILE);
    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fclose(stream: *mut FILE) -> c_int;
    pub fn fread(ptr: *mut c_void, size: size_t, nmemb: size_t, stream: *mut FILE) -> size_t;
    pub fn fwrite(ptr: *const c_void, size: size_t, nmemb: size_t, stream: *mut FILE) -> size_t;
    pub fn fseek(stream: *mut FILE, offset: c_long, whence: c_int) -> c_int;
    pub fn ftell(stream: *mut FILE) -> c_long;
    pub fn rewind(stream: *mut FILE);
    pub fn fileno(stream: *mut FILE) -> c_int;
    pub fn setbuf(stream: *mut FILE, buf: *mut c_char);
    pub fn setvbuf(stream: *mut FILE, buf: *mut c_char, mode: c_int, size: size_t) -> c_int;
    pub fn freopen(path: *const c_char, mode: *const c_char, stream: *mut FILE) -> *mut FILE;
    pub fn sscanf(s: *const c_char, format: *const c_char, ...) -> c_int;

    // --- termios ---
    pub fn tcgetattr(fd: c_int, termios_p: *mut termios) -> c_int;
    pub fn tcsetattr(fd: c_int, optional_actions: c_int, termios_p: *const termios) -> c_int;
    pub fn cfgetispeed(termios_p: *const termios) -> u32;
    pub fn cfgetospeed(termios_p: *const termios) -> u32;
    pub fn cfsetispeed(termios_p: *mut termios, speed: u32) -> c_int;
    pub fn cfsetospeed(termios_p: *mut termios, speed: u32) -> c_int;

    // --- Locale stubs ---
    pub fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    pub fn localeconv() -> *mut c_void;

    // --- ctype ---
    pub fn isalpha(c: c_int) -> c_int;
    pub fn isdigit(c: c_int) -> c_int;
    pub fn isalnum(c: c_int) -> c_int;
    pub fn isspace(c: c_int) -> c_int;
    pub fn isupper(c: c_int) -> c_int;
    pub fn islower(c: c_int) -> c_int;
    pub fn isprint(c: c_int) -> c_int;
    pub fn iscntrl(c: c_int) -> c_int;
    pub fn ispunct(c: c_int) -> c_int;
    pub fn isxdigit(c: c_int) -> c_int;
    pub fn isgraph(c: c_int) -> c_int;
    pub fn isblank(c: c_int) -> c_int;
    pub fn toupper(c: c_int) -> c_int;
    pub fn tolower(c: c_int) -> c_int;

    // --- wchar stubs ---
    pub fn wcscmp(s1: *const wchar_t, s2: *const wchar_t) -> c_int;
    pub fn wcslen(s: *const wchar_t) -> size_t;
    pub fn wcschr(s: *const wchar_t, c: wchar_t) -> *mut wchar_t;
    pub fn wcsrchr(s: *const wchar_t, c: wchar_t) -> *mut wchar_t;
    pub fn wmemchr(s: *const wchar_t, c: wchar_t, n: size_t) -> *mut wchar_t;
    pub fn wmemcmp(s1: *const wchar_t, s2: *const wchar_t, n: size_t) -> c_int;
    pub fn wmemcpy(dst: *mut wchar_t, src: *const wchar_t, n: size_t) -> *mut wchar_t;
    pub fn wmemset(s: *mut wchar_t, c: wchar_t, n: size_t) -> *mut wchar_t;
    pub fn mbrtowc(
        pwc: *mut wchar_t,
        s: *const c_char,
        n: size_t,
        ps: *mut c_void,
    ) -> size_t;
    pub fn wcrtomb(s: *mut c_char, wc: wchar_t, ps: *mut c_void) -> size_t;

    // --- Standard streams ---
    pub static stdin: *mut FILE;
    pub static stdout: *mut FILE;
    pub static stderr: *mut FILE;
    pub static mut environ: *mut *mut c_char;
}

// CMSG helper macros as functions
#[inline]
pub fn CMSG_ALIGN(len: size_t) -> size_t {
    (len + core::mem::size_of::<size_t>() - 1) & !(core::mem::size_of::<size_t>() - 1)
}

#[inline]
pub unsafe fn CMSG_FIRSTHDR(mhdr: *const msghdr) -> *mut cmsghdr {
    unsafe {
        if (*mhdr).msg_controllen >= core::mem::size_of::<cmsghdr>() {
            (*mhdr).msg_control as *mut cmsghdr
        } else {
            core::ptr::null_mut()
        }
    }
}

#[inline]
pub unsafe fn CMSG_NXTHDR(mhdr: *const msghdr, cmsg: *const cmsghdr) -> *mut cmsghdr {
    unsafe {
        let next = (cmsg as *const u8).add(CMSG_ALIGN((*cmsg).cmsg_len)) as *mut cmsghdr;
        let end = ((*mhdr).msg_control as *const u8).add((*mhdr).msg_controllen);
        if (next as *const u8).add(core::mem::size_of::<cmsghdr>()) > end {
            core::ptr::null_mut()
        } else {
            next
        }
    }
}

#[inline]
pub unsafe fn CMSG_DATA(cmsg: *const cmsghdr) -> *mut u8 {
    (cmsg as *mut u8).add(CMSG_ALIGN(core::mem::size_of::<cmsghdr>()))
}

#[inline]
pub fn CMSG_SPACE(length: size_t) -> size_t {
    CMSG_ALIGN(core::mem::size_of::<cmsghdr>()) + CMSG_ALIGN(length)
}

#[inline]
pub fn CMSG_LEN(length: size_t) -> size_t {
    CMSG_ALIGN(core::mem::size_of::<cmsghdr>()) + length
}

// FD_SET/FD_CLR/FD_ISSET/FD_ZERO helper functions
#[inline]
pub unsafe fn FD_SET(fd: c_int, set: *mut fd_set) {
    unsafe {
        let fd = fd as usize;
        (*set).fds_bits[fd / 64] |= 1u64 << (fd % 64);
    }
}

#[inline]
pub unsafe fn FD_CLR(fd: c_int, set: *mut fd_set) {
    unsafe {
        let fd = fd as usize;
        (*set).fds_bits[fd / 64] &= !(1u64 << (fd % 64));
    }
}

#[inline]
pub unsafe fn FD_ISSET(fd: c_int, set: *const fd_set) -> bool {
    unsafe {
        let fd = fd as usize;
        ((*set).fds_bits[fd / 64] & (1u64 << (fd % 64))) != 0
    }
}

#[inline]
pub unsafe fn FD_ZERO(set: *mut fd_set) {
    unsafe {
        let mut i = 0;
        while i < FD_SETSIZE / 64 {
            (*set).fds_bits[i] = 0;
            i += 1;
        }
    }
}
