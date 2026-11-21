/* T1622: original-child ownership, cleanup and injected native failures. */
#define _GNU_SOURCE
#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <ftw.h>
#include <signal.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <unistd.h>

enum fault { NONE, MASK, ACTION, ALLOCATE, CREATE, OPEN, CLOSE, FORK, WAIT, SIGNAL, KILL, STAT, LSTAT, WALK, REMOVE };
static enum fault injected;
static bool fault(enum fault point)
{
    if (injected != point)
    {
        return false;
    }
    injected = NONE;
    errno = EIO;
    return true;
}

static int test_mask(int how, const sigset_t *set, sigset_t *old)
{
    return fault(MASK) ? -1 : sigprocmask(how, set, old);
}

static int test_action(int sig, const struct sigaction *action, struct sigaction *old)
{
    return fault(ACTION) ? -1 : sigaction(sig, action, old);
}

static int test_close(int fd)
{
    int result = close(fd);
    return fault(CLOSE) ? -1 : result;
}

static void *test_allocate(size_t size)
{
    return fault(ALLOCATE) ? NULL : malloc(size);
}

static char *test_create(char *path)
{
    return fault(CREATE) ? NULL : mkdtemp(path);
}

static int test_open(const char *path, int flags)
{
    return fault(OPEN) ? -1 : open(path, flags);
}

static pid_t test_fork(void)
{
    return fault(FORK) ? -1 : fork();
}

static pid_t test_wait(pid_t pid, int *status, int options)
{
    return fault(WAIT) ? -1 : waitpid(pid, status, options);
}

static int test_signal(const sigset_t *set, siginfo_t *info)
{
    return fault(SIGNAL) ? -1 : sigwaitinfo(set, info);
}

static int test_kill(pid_t pid, int sig)
{
    return fault(KILL) ? -1 : kill(pid, sig);
}

static int test_stat(int fd, struct stat *metadata)
{
    return fault(STAT) ? -1 : fstat(fd, metadata);
}

static int test_lstat(const char *path, struct stat *metadata)
{
    return fault(LSTAT) ? -1 : lstat(path, metadata);
}

static int test_walk(const char *path, int (*callback)(const char *, const struct stat *, int, struct FTW *), int count, int flags)
{
    return fault(WALK) ? -1 : nftw(path, callback, count, flags);
}

static int test_remove(const char *path)
{
    return fault(REMOVE) ? -1 : rmdir(path);
}

#define sigprocmask test_mask
#define sigaction(...) test_action(__VA_ARGS__)
#define close test_close
#define malloc test_allocate
#define mkdtemp test_create
#define open test_open
#define fork test_fork
#define waitpid test_wait
#define sigwaitinfo test_signal
#define kill test_kill
#define fstat test_stat
#define lstat test_lstat
#define nftw test_walk
#define rmdir test_remove
#include "../../packaging/appimage/runtime/extraction.c"
#undef kill
#undef close
#undef rmdir
#undef sigprocmask
#undef sigaction

static const char *temporary;

static struct blent_extraction begin(void)
{
    struct blent_extraction owner = {0};
    assert(blent_extraction_begin(&owner, temporary) == 0);
    assert(owner.phase == BLENT_EXTRACTION_READY);
    return owner; /* Test-only transfer before any borrow or fork. */
}

static void invalid_states(void)
{
    struct blent_extraction owner = {0};
    assert(blent_extraction_spawn(&owner) == -1);
    assert(blent_extraction_child(&owner) == -1);
    assert(blent_extraction_wait(&owner) == 127);
    assert(blent_extraction_finish(&owner, 0) == 127);
    assert(blent_extraction_begin(&owner, NULL) == -1);
    assert(blent_extraction_begin(&owner, temporary) == -1);
    owner = (struct blent_extraction){0};
    assert(blent_extraction_begin(&owner, "") == -1);
    owner = (struct blent_extraction){0};
    assert(blent_extraction_begin(&owner, "/nonexistent/blent-t1622") == -1);
}

static void begin_faults(void)
{
    const enum fault points[] = {MASK, ACTION, ALLOCATE, CREATE, OPEN};
    for (size_t i = 0; i < sizeof(points) / sizeof(points[0]); ++i)
    {
        struct blent_extraction owner = {0};
        injected = points[i];
        assert(blent_extraction_begin(&owner, temporary) == -1);
        assert(injected == NONE);
    }
}

static void cleanup_entries(void)
{
    struct blent_extraction owner = begin();
    char child[4096];
    snprintf(child, sizeof(child), "%s/file", owner.path);
    FILE *file = fopen(child, "w");
    assert(file != NULL);
    assert(fclose(file) == 0);
    snprintf(child, sizeof(child), "%s/link", owner.path);
    assert(symlink(temporary, child) == 0);
    snprintf(child, sizeof(child), "%s/dangling", owner.path);
    assert(symlink("absent", child) == 0);
    snprintf(child, sizeof(child), "%s/directory", owner.path);
    assert(mkdir(child, 0700) == 0);
    assert(blent_extraction_finish(&owner, 7) == 7);
    assert(owner.path == NULL);
    assert(owner.directory == -1);
    assert(blent_extraction_finish(&owner, 0) == 127);
    assert(extraction_remove_entry("unused", NULL, FTW_NS, NULL) == -1);
}

static void cleanup_faults(void)
{
    const enum fault points[] = {STAT, LSTAT, WALK, REMOVE, CLOSE, ACTION, MASK};
    for (size_t i = 0; i < sizeof(points) / sizeof(points[0]); ++i)
    {
        struct blent_extraction owner = begin();
        char *path = strdup(owner.path);
        injected = points[i];
        assert(blent_extraction_finish(&owner, 0) == 127);
        assert(injected == NONE);
        rmdir(path);
        free(path);
    }
    /* An injected restore failure must not leak a blocked mask into the harness. */
    sigset_t empty;
    sigemptyset(&empty);
    assert(sigprocmask(SIG_SETMASK, &empty, NULL) == 0);
}

static void replaced_identity(void)
{
    struct blent_extraction owner = begin();
    char *old = strdup(owner.path);
    char moved[4096];
    snprintf(moved, sizeof(moved), "%s-moved", old);
    assert(rename(old, moved) == 0);
    assert(mkdir(old, 0700) == 0);
    assert(blent_extraction_finish(&owner, 0) == 127);
    assert(rmdir(old) == 0);
    assert(rmdir(moved) == 0);
    free(old);
}

static void retained_choice(void)
{
    struct blent_extraction owner = begin();
    char *path = strdup(owner.path);
    assert(setenv("NO_CLEANUP", "1", 1) == 0);
    assert(blent_extraction_finish(&owner, 0) == 0);
    assert(unsetenv("NO_CLEANUP") == 0);
    assert(rmdir(path) == 0);
    free(path);
}

static void child_case(enum fault point, bool signalled)
{
    struct blent_extraction owner = begin();
    pid_t child = blent_extraction_spawn(&owner);
    assert(child >= 0);
    if (child == 0)
    {
        assert(blent_extraction_child(&owner) == 0);
        usleep(50000);
        exit(11);
    }
    assert(blent_extraction_finish(&owner, 0) == 127);
    injected = point;
    if (signalled)
    {
        assert(kill(getpid(), SIGTERM) == 0);
    }
    int status = blent_extraction_wait(&owner);
    assert(status == 11 || status == 127);
    assert(injected == NONE);
    assert(blent_extraction_finish(&owner, status) == status);
}

static void fork_failure(void)
{
    struct blent_extraction owner = begin();
    injected = FORK;
    assert(blent_extraction_spawn(&owner) == -1);
    assert(blent_extraction_finish(&owner, 0) == 0);
}

static void child_release_failure(void)
{
    struct blent_extraction owner = begin();
    pid_t child = blent_extraction_spawn(&owner);
    assert(child >= 0);
    if (child == 0)
    {
        injected = CLOSE;
        exit(blent_extraction_child(&owner) == -1 ? 0 : 1);
    }
    assert(blent_extraction_wait(&owner) == 0);
    assert(blent_extraction_finish(&owner, 0) == 0);
}

int main(int argc, char **argv)
{
    assert(argc == 2);
    temporary = argv[1];
    unsetenv("NO_CLEANUP");
    invalid_states();
    begin_faults();
    cleanup_entries();
    cleanup_faults();
    replaced_identity();
    retained_choice();
    fork_failure();
    child_release_failure();
    child_case(NONE, false);
    child_case(NONE, true);
    child_case(WAIT, false);
    child_case(SIGNAL, false);
    child_case(KILL, true);
    return 0;
}
