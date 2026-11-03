/* Trusted static fixture only; not a compositor or an isolation certificate. */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/prctl.h>
#include <sys/resource.h>
#include <unistd.h>

extern char **environ;

static void check_control(const char *root, const char *name, const char *expected) {
    char path[4096], text[256];
    if (snprintf(path, sizeof(path), "%s/%s", root, name) >= (int)sizeof(path)) _exit(80);
    FILE *file = fopen(path, "r");
    if (!file || !fgets(text, sizeof(text), file) || fclose(file)) _exit(81);
    text[strcspn(text, "\n")] = 0;
    if (strcmp(text, expected)) _exit(82);
}

int main(int argc, char **argv) {
    if (argc != 3 || environ[0] != NULL) return 83;
    if (prctl(PR_GET_NO_NEW_PRIVS, 0, 0, 0, 0) != 1) return 84;
    struct rlimit core;
    if (getrlimit(RLIMIT_CORE, &core) || core.rlim_cur || core.rlim_max) return 85;
    for (int fd = 4; fd < 65536; ++fd) {
        errno = 0;
        if (fcntl(fd, F_GETFD) >= 0 || errno != EBADF) return 86;
    }
    sigset_t mask;
    struct sigaction action;
    if (sigprocmask(SIG_SETMASK, NULL, &mask) || sigismember(&mask, SIGUSR1)) return 87;
    if (sigaction(SIGUSR1, NULL, &action) || action.sa_handler != SIG_DFL) return 88;
    check_control(argv[1], "memory.max", "536870912");
    check_control(argv[1], "memory.swap.max", "0");
    check_control(argv[1], "memory.oom.group", "1");
    check_control(argv[1], "pids.max", "64");
    check_control(argv[1], "cpu.max", "100000 100000");
    char pid[32];
    snprintf(pid, sizeof(pid), "%ld", (long)getpid());
    check_control(argv[1], "cgroup.procs", pid);
    const uint32_t record[3] = {0x48594547, (uint32_t)getpid(), 1};
    if (write(3, record, sizeof(record)) != (ssize_t)sizeof(record)) return 89;
    if (!strcmp(argv[2], "wait")) for (;;) pause();
    return strcmp(argv[2], "exit") ? 90 : 0;
}
