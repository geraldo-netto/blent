/* T774: parse errors must fail before any benchmark resource is opened. */
#define BLENT_EXCHANGE_GENERATION
#define main benchmark_main
#include "../../scripts/benchmarks/conversion.c"
#undef main
#include <sys/wait.h>
#include <unistd.h>

static void reject(const char *text) {
    pid_t child = fork(); assert(child >= 0);
    if (!child) {
        char *args[] = {"benchmark", (char *)text, "1", "1", "0", "1"};
        int sessions, workers, scale, damage, samples;
        arguments(6, args, &sessions, &workers, &scale, &damage, &samples);
        _exit(0);
    }
    int status; assert(waitpid(child, &status, 0) == child);
    assert(WIFEXITED(status) && WEXITSTATUS(status) == 2);
}

int main(void) {
    const char *invalid[] = {"1tail", "", "junk", "2147483648", "-2147483649", "99999999999999999999999999"};
    for (unsigned i = 0; i < sizeof(invalid) / sizeof(invalid[0]); i++) reject(invalid[i]);
    return 0;
}
