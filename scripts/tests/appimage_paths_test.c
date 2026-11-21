/* T1639: byte-exact boundaries and invalid path lengths. */
#define _GNU_SOURCE
#include <assert.h>
#include <errno.h>
#include <string.h>
#include <stdlib.h>
#include <sys/wait.h>
#include <unistd.h>
#include "../../packaging/appimage/runtime/paths.c"

static void ranges(void)
{
    char base[24];
    memset(base, 'b', sizeof(base));
    base[23] = '\0';
    for (size_t length = 0; length < sizeof(base); ++length)
    {
        base[length] = '\0';
        for (size_t capacity = 0; capacity <= 24; ++capacity)
        {
            char output[26];
            memset(output, 'x', sizeof(output));
            int result = blent_path(output + 1, capacity, base, ".end");
            assert((result == 0) == (length + 4 < capacity));
            assert(output[0] == 'x');
            assert(output[capacity + 1] == 'x');
        }
        base[length] = 'b';
    }
}

static void required(void)
{
    char output[8];
    blent_path_required(output, sizeof(output), "abc", ".end");
    assert(strcmp(output, "abc.end") == 0);
    pid_t child = fork();
    assert(child >= 0);
    if (child == 0)
    {
        blent_path_required(output, sizeof(output), "abcd", ".end");
        exit(1);
    }
    int status;
    assert(waitpid(child, &status, 0) == child);
    assert(WIFEXITED(status) && WEXITSTATUS(status) == 127);
}

static void links(const char *root)
{
    char path[4096];
    assert(snprintf(path, sizeof(path), "%s/link", root) > 0);
    assert(symlink("abcd", path) == 0);
    char output[8];
    assert(blent_readlink(path, output, 5) == 4);
    assert(strcmp(output, "abcd") == 0);
    assert(blent_readlink(path, output, 4) == -1);
    assert(errno == ENAMETOOLONG);
    assert(blent_readlink(path, output, 0) == -1);
    assert(unlink(path) == 0);
    assert(blent_readlink(path, output, sizeof(output)) == -1);
}

int main(int argc, char **argv)
{
    assert(argc == 2);
    ranges();
    required();
    links(argv[1]);
    return 0;
}
