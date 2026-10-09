/* T777: older damage replay must enforce the shared numeric contract. */
#define BLENT_EXCHANGE_GENERATION
#define main benchmark_main
#include "../../scripts/benchmarks/damage_regions.c"
#undef main
#include <sys/wait.h>
#include <unistd.h>

static void t777_reject(const char *text, int field) {
    pid_t child = fork(); assert(child >= 0);
    if (!child) {
        char *args[] = {"benchmark", "1", "0"}; args[field] = (char *)text;
        _exit(benchmark_main(3, args));
    }
    int status; assert(waitpid(child, &status, 0) == child);
    assert(WIFEXITED(status) && WEXITSTATUS(status) == 2 && "T777: reject malformed and out-of-range numbers");
}
int main(void) {
    const char *invalid[] = {"1tail", "", "junk", "2147483648", "-2147483649", "99999999999999999999999999", "-1", "5"};
    for (unsigned i = 0; i < sizeof(invalid) / sizeof(invalid[0]); i++)
        for (int field = 1; field <= 2; field++) t777_reject(invalid[i], field);
    t777_reject("0", 1);
    char *valid[] = {"benchmark", "4", "4"};
    assert(!benchmark_main(3, valid));
    return 0;
}
