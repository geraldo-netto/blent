/* T1286: successful argument validation reaches the first resource admission. */
#include <pthread.h>
#include <stdlib.h>
#include <sys/socket.h>

int __wrap_pthread_condattr_init(pthread_condattr_t *attributes)
{
    (void)attributes;
    exit(77);
}

int __wrap_socketpair(int domain, int type, int protocol, int sockets[2])
{
    (void)domain;
    (void)type;
    (void)protocol;
    (void)sockets;
    exit(77);
}
