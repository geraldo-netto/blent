/* Blent-owned extraction lifecycle adapter for type2-runtime 20251108. */
#ifndef BLENT_EXTRACTION_H
#define BLENT_EXTRACTION_H

#include <signal.h>
#include <sys/types.h>

enum blent_extraction_phase
{
    BLENT_EXTRACTION_EMPTY,
    BLENT_EXTRACTION_READY,
    BLENT_EXTRACTION_CHILD,
    BLENT_EXTRACTION_RUNNING,
    BLENT_EXTRACTION_SETTLED,
    BLENT_EXTRACTION_CLOSED
};

/* One process-entry owner. Initialize to zero; never copy after admission.
 * The child receives only its fork copy and retires its descriptor before exec.
 * watched signals stay blocked in the parent through exact reap and cleanup. */
struct blent_extraction
{
    char *path;
    int directory;
    pid_t child;
    enum blent_extraction_phase phase;
    sigset_t watched;
    sigset_t previous_mask;
    struct sigaction previous_child;
};

int blent_extraction_begin(struct blent_extraction *owner, const char *temporary);
pid_t blent_extraction_spawn(struct blent_extraction *owner);
int blent_extraction_child(struct blent_extraction *owner);
int blent_extraction_wait(struct blent_extraction *owner);
int blent_extraction_finish(struct blent_extraction *owner, int status);

#endif
