#define _GNU_SOURCE
#include "extraction.h"
#include <errno.h>
#include <fcntl.h>
#include <ftw.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <unistd.h>

/* Current Blent bundles admit at most 63 nested directories. The pinned musl
 * walker uses one descriptor per level. Failed cleanup is reported, never
 * mistaken for a successful release. The root pin is one additional descriptor. */
#define BLENT_CLEANUP_DESCRIPTORS 64
#define BLENT_EXEC_ERROR 127

static int extraction_signals(struct blent_extraction *owner)
{
    sigemptyset(&owner->watched);
    sigaddset(&owner->watched, SIGTERM);
    sigaddset(&owner->watched, SIGINT);
    sigaddset(&owner->watched, SIGHUP);
    sigaddset(&owner->watched, SIGCHLD);
    if (sigprocmask(SIG_BLOCK, &owner->watched, &owner->previous_mask) < 0)
    {
        return -1;
    }
    struct sigaction action = {.sa_handler = SIG_DFL};
    sigemptyset(&action.sa_mask);
    if (sigaction(SIGCHLD, &action, &owner->previous_child) < 0)
    {
        sigprocmask(SIG_SETMASK, &owner->previous_mask, NULL);
        return -1;
    }
    return 0;
}

static int extraction_restore(struct blent_extraction *owner)
{
    int action = sigaction(SIGCHLD, &owner->previous_child, NULL);
    int mask = sigprocmask(SIG_SETMASK, &owner->previous_mask, NULL);
    return action < 0 || mask < 0 ? -1 : 0;
}

static char *extraction_directory(const char *temporary)
{
    static const char suffix[] = "/blent-appimage-XXXXXX";
    if (temporary == NULL || temporary[0] == '\0')
    {
        errno = EINVAL;
        return NULL;
    }
    size_t length = strlen(temporary);
    if (length > SIZE_MAX - sizeof(suffix))
    {
        errno = EOVERFLOW;
        return NULL;
    }
    char *path = malloc(length + sizeof(suffix));
    if (path == NULL)
    {
        return NULL;
    }
    memcpy(path, temporary, length);
    memcpy(path + length, suffix, sizeof(suffix));
    if (mkdtemp(path) == NULL)
    {
        free(path);
        return NULL;
    }
    return path;
}

static int extraction_release(struct blent_extraction *owner)
{
    int result = 0;
    if (owner->directory >= 0)
    {
        result = close(owner->directory);
        owner->directory = -1;
    }
    free(owner->path);
    owner->path = NULL;
    owner->phase = BLENT_EXTRACTION_CLOSED;
    int restored = extraction_restore(owner);
    return result < 0 || restored < 0 ? -1 : 0;
}

int blent_extraction_begin(struct blent_extraction *owner, const char *temporary)
{
    if (owner->phase != BLENT_EXTRACTION_EMPTY)
    {
        errno = EINVAL;
        return -1;
    }
    owner->directory = -1;
    if (extraction_signals(owner) < 0)
    {
        return -1;
    }
    owner->path = extraction_directory(temporary);
    if (owner->path == NULL)
    {
        int failure = errno;
        extraction_release(owner);
        errno = failure;
        return -1;
    }
    owner->directory = open(owner->path, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    if (owner->directory < 0)
    {
        int failure = errno;
        if (rmdir(owner->path) < 0)
        {
            fprintf(stderr, "Extraction directory retained after open failure: %s\n", owner->path);
        }
        extraction_release(owner);
        errno = failure;
        return -1;
    }
    owner->phase = BLENT_EXTRACTION_READY;
    return 0;
}

pid_t blent_extraction_spawn(struct blent_extraction *owner)
{
    if (owner->phase != BLENT_EXTRACTION_READY)
    {
        errno = EINVAL;
        return -1;
    }
    pid_t child = fork();
    if (child >= 0)
    {
        owner->child = child;
        owner->phase = child == 0 ? BLENT_EXTRACTION_CHILD : BLENT_EXTRACTION_RUNNING;
    }
    return child;
}

int blent_extraction_child(struct blent_extraction *owner)
{
    if (owner->phase != BLENT_EXTRACTION_CHILD)
    {
        errno = EINVAL;
        return -1;
    }
    int result = close(owner->directory);
    owner->directory = -1;
    int restored = extraction_restore(owner);
    return result < 0 || restored < 0 ? -1 : 0;
}

static void extraction_wait_error(const char *operation, bool *reported)
{
    if (!*reported)
    {
        perror(operation);
        *reported = true;
    }
    sleep(1);
}

static void extraction_signal(struct blent_extraction *owner, bool *reported)
{
    int event = sigwaitinfo(&owner->watched, NULL);
    if (event == SIGTERM || event == SIGINT || event == SIGHUP)
    {
        /* No other waiter exists, and signals remain blocked. An exited child
         * remains a zombie until our next poll, so its PID cannot be reused. */
        if (kill(owner->child, event) < 0 && errno != ESRCH)
        {
            extraction_wait_error("forward extraction stop", reported);
        }
    }
    else if (event < 0 && errno != EINTR)
    {
        extraction_wait_error("wait for extraction signal", reported);
    }
}

int blent_extraction_wait(struct blent_extraction *owner)
{
    if (owner->phase != BLENT_EXTRACTION_RUNNING)
    {
        return BLENT_EXEC_ERROR;
    }
    bool reported = false;
    for (;;)
    {
        int status = 0;
        pid_t result = waitpid(owner->child, &status, WNOHANG);
        if (result == owner->child)
        {
            owner->child = 0;
            owner->phase = BLENT_EXTRACTION_SETTLED;
            return WIFEXITED(status) ? WEXITSTATUS(status) : BLENT_EXEC_ERROR;
        }
        if (result < 0)
        {
            extraction_wait_error("observe original extraction child; retaining directory", &reported);
            continue;
        }
        extraction_signal(owner, &reported);
    }
}

static int extraction_remove_entry(const char *path, const struct stat *metadata,
                                   int kind, struct FTW *position)
{
    (void)metadata;
    (void)position;
    if (kind == FTW_DP)
    {
        return rmdir(path);
    }
    if (kind == FTW_F || kind == FTW_SL || kind == FTW_SLN)
    {
        return unlink(path);
    }
    errno = EIO;
    return -1;
}

static int extraction_remove(struct blent_extraction *owner)
{
    struct stat pinned;
    struct stat named;
    if (fstat(owner->directory, &pinned) < 0 || lstat(owner->path, &named) < 0)
    {
        return -1;
    }
    if (pinned.st_dev != named.st_dev || pinned.st_ino != named.st_ino)
    {
        errno = ESTALE;
        return -1;
    }
    return nftw(owner->path, extraction_remove_entry, BLENT_CLEANUP_DESCRIPTORS,
                FTW_DEPTH | FTW_MOUNT | FTW_PHYS);
}

int blent_extraction_finish(struct blent_extraction *owner, int status)
{
    if (owner->phase != BLENT_EXTRACTION_READY && owner->phase != BLENT_EXTRACTION_SETTLED)
    {
        return BLENT_EXEC_ERROR;
    }
    if (getenv("NO_CLEANUP") != NULL)
    {
        fprintf(stderr, "Extraction retained by NO_CLEANUP: %s\n", owner->path);
    }
    else if (extraction_remove(owner) < 0)
    {
        fprintf(stderr, "Extraction cleanup failed; directory retained at %s: %s\n", owner->path, strerror(errno));
        status = BLENT_EXEC_ERROR;
    }
    if (extraction_release(owner) < 0)
    {
        status = BLENT_EXEC_ERROR;
    }
    return status;
}
