/* Trusted static CI fixture, not a compositor or an imported program. */
#define _GNU_SOURCE
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <sched.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mount.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/statfs.h>
#include <sys/syscall.h>
#include <sys/utsname.h>
#include <sys/un.h>
#include <unistd.h>

extern char **environ;
#define CHECK(test) do { if (!(test)) { fprintf(stderr, "Boundary assertion failed at line %d: %s (errno %d)\n", __LINE__, #test, errno); return 1; } } while (0)
#define DENIED(call) do { errno = 0; CHECK((call) == -1 && errno == EPERM); } while (0)
static int read_text(const char *path, char *data, size_t size) {
    int fd = open(path, O_RDONLY | O_CLOEXEC);
    if (fd < 0) return -1;
    ssize_t count = read(fd, data, size - 1);
    close(fd);
    if (count < 0 || (size_t)count >= size - 1) return -1;
    data[count] = '\0'; return 0;
}
static int absent(const char *path) {
    struct stat status;
    errno = 0;
    return lstat(path, &status) == -1 && errno == ENOENT;
}
int main(int argc, char **argv) {
    CHECK(argc == 10);
    CHECK(getuid() == 1000 && geteuid() == 1000 && getgid() == 1000 && getegid() == 1000);
    CHECK(getgroups(0, NULL) == 0);
    const char *names[] = {"user", "mnt", "pid", "net", "ipc", "uts", "cgroup"};
    struct stat status;
    for (size_t i = 0; i < 7; ++i) {
        char path[64]; snprintf(path, sizeof(path), "/proc/self/ns/%s", names[i]);
        CHECK(stat(path, &status) == 0);
        CHECK(status.st_ino != strtoull(argv[i+3], NULL, 10));
        printf("HYE_RUNTIME_NAMESPACE %s %llu\n", names[i], (unsigned long long)status.st_ino);
    }
    char text[16384];
    CHECK(read_text("/proc/self/status", text, sizeof(text)) == 0);
    CHECK(strstr(text, "NoNewPrivs:\t1\n") != NULL);
    CHECK(strstr(text, "Seccomp:\t2\n") != NULL);
    for (size_t i = 0; i < 4; ++i) {
        const char *caps[] = {"CapPrm:", "CapEff:", "CapBnd:", "CapAmb:"};
        const char *row = strstr(text, caps[i]);
        CHECK(row != NULL && strtoull(row + strlen(caps[i]), NULL, 16) == 0);
    }
    CHECK(read_text("/proc/self/uid_map", text, sizeof(text)) == 0);
    printf("HYE_RUNTIME_UID_MAP %s", text);
    CHECK(read_text("/proc/self/gid_map", text, sizeof(text)) == 0);
    printf("HYE_RUNTIME_GID_MAP %s", text);
    struct utsname uts;
    CHECK(uname(&uts) == 0 && strcmp(uts.nodename, "hye-lab") == 0);
    CHECK(getpid() == 2 && getppid() == 1);
    DIR *proc = opendir("/proc"); CHECK(proc != NULL);
    struct dirent *entry;
    int processes = 0;
    while ((entry = readdir(proc)) != NULL) {
        char *end; long pid = strtol(entry->d_name, &end, 10);
        if (*entry->d_name && !*end) { CHECK(pid == 1 || pid == 2); ++processes; }
    }
    closedir(proc); CHECK(processes == 2);
    for (int fd = 3; fd < 4096; ++fd) { errno = 0; CHECK(fcntl(fd, F_GETFD) == -1 && errno == EBADF); }
    CHECK(absent(argv[1]));
    for (size_t i = 0; i < 10; ++i) {
        const char *paths[] = {"/sys", "/dev/input", "/dev/uinput", "/dev/dri", "/run/user", "/etc/passwd", "/etc/shadow", "/home/runner", "/usr/bin/sh", "/proc/1/root/etc/passwd"};
        CHECK(absent(paths[i]));
    }
    CHECK(read_text("/usr/share/hye/canary", text, sizeof(text)) == 0 && strcmp(text, "reviewed-runtime") == 0);
    errno = 0; CHECK(open("/usr/share/hye/canary", O_WRONLY) == -1 && errno == EROFS);
    errno = 0; CHECK(open("/outside", O_WRONLY | O_CREAT, 0600) == -1 && errno == EROFS);
    const char *writable[] = {"/tmp", "/run/lab", "/home/lab", "/state/config", "/state/data", "/state/cache", "/dev/shm"};
    for (size_t i = 0; i < 7; ++i) {
        char path[128]; snprintf(path, sizeof(path), "%s/probe", writable[i]);
        int fd = open(path, O_WRONLY | O_CREAT | O_EXCL | O_CLOEXEC, 0600);
        CHECK(fd >= 0 && write(fd, "x", 1) == 1); close(fd);
        CHECK(unlinkat(AT_FDCWD, path, 0) == 0);
    }
    const char *mounts[] = {"/tmp", "/run", "/home", "/state"};
    const unsigned long long sizes[] = {33554432, 16777216, 16777216, 33554432};
    for (size_t i = 0; i < 4; ++i) {
        struct statfs fs;
        CHECK(statfs(mounts[i], &fs) == 0 && fs.f_type == 0x01021994);
        CHECK((unsigned long long)fs.f_bsize * fs.f_blocks <= sizes[i]);
    }
    size_t count = 0; while (environ[count]) ++count; CHECK(count == 7);
    const char *keys[] = {"HOME", "XDG_RUNTIME_DIR", "XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_CACHE_HOME", "LC_ALL", "TMPDIR"};
    const char *values[] = {"/home/lab", "/run/lab", "/state/config", "/state/data", "/state/cache", "C", "/tmp"};
    for (size_t i = 0; i < 7; ++i) CHECK(getenv(keys[i]) && strcmp(getenv(keys[i]), values[i]) == 0);
    int unix_socket = socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC, 0); CHECK(unix_socket >= 0);
    struct sockaddr_un address = {.sun_family = AF_UNIX};
    CHECK(strlen(argv[2]) < sizeof(address.sun_path) - 1);
    memcpy(address.sun_path + 1, argv[2], strlen(argv[2]));
    errno = 0;
    CHECK(connect(unix_socket, (struct sockaddr *)&address, offsetof(struct sockaddr_un, sun_path) + 1 + strlen(argv[2])) == -1 && errno == ECONNREFUSED);
    close(unix_socket);
    DENIED(socket(AF_INET, SOCK_STREAM, 0)); DENIED(socket(AF_INET6, SOCK_STREAM, 0));
    DENIED(socket(AF_NETLINK, SOCK_RAW, 0));
    DENIED(syscall(SYS_ptrace, 0, 0, 0, 0));
    DENIED(syscall(SYS_process_vm_readv, getpid(), NULL, 0, NULL, 0, 0));
    DENIED(syscall(SYS_pidfd_open, getpid(), 0));
    DENIED(syscall(SYS_pidfd_getfd, -1, 0, 0));
    DENIED(syscall(SYS_pidfd_send_signal, -1, 0, NULL, 0));
    DENIED(syscall(SYS_unshare, CLONE_NEWUSER));
    DENIED(syscall(SYS_setns, -1, 0));
    DENIED(syscall(SYS_mount, NULL, "/tmp", "tmpfs", 0, NULL));
    DENIED(syscall(SYS_io_uring_setup, 0, NULL));
    DENIED(syscall(SYS_bpf, -1, NULL, 0));
    DENIED(syscall(SYS_execveat, -1, "", NULL, NULL, 0));
    DENIED(syscall(SYS_clone3, NULL, 0));
    puts("HYE_RUNTIME_RESULT all boundary assertions passed");
    return 0;
}
