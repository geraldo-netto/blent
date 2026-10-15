#define _GNU_SOURCE
#include "capture_control.h"
#include <errno.h>
#include <fcntl.h>
#include <sys/socket.h>
#include <sys/un.h>

static int capture_control_socket(int fd)
{
    int kind = 0;
    socklen_t size = sizeof(kind);
    if (getsockopt(fd, SOL_SOCKET, SO_TYPE, &kind, &size) != 0 || kind != SOCK_SEQPACKET)
        return 0;
    struct sockaddr_un peer = {0};
    size = sizeof(peer);
    return getpeername(fd, (struct sockaddr *)&peer, &size) == 0 && peer.sun_family == AF_UNIX;
}

int capture_control_init(capture_control_t *control, int fd)
{
    if (fd < 3 || !capture_control_socket(fd))
        return 0;
    int flags = fcntl(fd, F_GETFL);
    if (flags < 0 || fcntl(fd, F_SETFL, flags | O_NONBLOCK) != 0)
        return 0;
    *control = (capture_control_t){.fd = fd};
    return 1;
}

int capture_control_poll(capture_control_t *control)
{
    if (control->current.sequence != control->acknowledged)
        return 0;
    unsigned char bytes[CAPTURE_ACTIVITY_BYTES];
    struct iovec vector = {bytes, sizeof(bytes)};
    /* No ancillary capacity: Linux closes discarded SCM_RIGHTS descriptors.
     * MSG_CTRUNC rejects any such packet, including one with a valid payload. */
    struct msghdr message = {.msg_iov = &vector, .msg_iovlen = 1};
    ssize_t count = recvmsg(control->fd, &message, MSG_DONTWAIT);
    if (count < 0)
        return errno == EAGAIN || errno == EINTR ? 0 : -1;
    if (message.msg_flags & (MSG_TRUNC | MSG_CTRUNC))
        return -1;
    capture_activity_t request;
    if (!capture_activity_decode(bytes, (size_t)count, &request))
        return -1;
    if (!capture_activity_next(request, control->current.sequence))
        return -1;
    control->current = request;
    return 1;
}

int capture_control_ack(capture_control_t *control)
{
    if (control->current.sequence == control->acknowledged)
        return 1;
    unsigned char bytes[CAPTURE_ACTIVITY_BYTES];
    capture_activity_encode(control->current, bytes);
    ssize_t count = send(control->fd, bytes, sizeof(bytes), MSG_DONTWAIT | MSG_NOSIGNAL);
    if (count < 0)
        return errno == EAGAIN || errno == EINTR ? 0 : -1;
    if (count != sizeof(bytes))
        return -1;
    control->acknowledged = control->current.sequence;
    return 1;
}
