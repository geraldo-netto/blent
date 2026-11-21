/* T1639: bounded runtime path operations. Inputs are valid C strings; output
 * holds capacity bytes and does not overlap either input. No stored views or allocations escape a call. */
#ifndef BLENT_RUNTIME_PATHS_H
#define BLENT_RUNTIME_PATHS_H
#include <stddef.h>
#include <sys/types.h>
int blent_path(char *output, size_t capacity, const char *base, const char *suffix);
void blent_path_required(char *output, size_t capacity, const char *base, const char *suffix);
ssize_t blent_readlink(const char *path, char *output, size_t capacity);
#endif
