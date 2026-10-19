"""T1137: private PTYs, original pidfds and cleanup before ownership assertions."""
import argparse
import array
import contextlib
import ctypes
import fcntl
import json
import os
from pathlib import Path
import pty
import select
import signal
import socket
import subprocess
import struct
import tempfile
import tty


def signal_original(fd):
    try:
        signal.pidfd_send_signal(fd, signal.SIGKILL)
    except ProcessLookupError:
        pass  # The same pinned original has already exited.


class Processes:
    def __init__(self):
        self.parent = None
        self.parent_fd = -1
        self.writer_fd = -1

    def stop_parent(self):
        if self.parent is None:
            return
        if self.parent_fd >= 0:
            signal_original(self.parent_fd)
        else:
            # Direct, unreaped Popen child: its original PID cannot be reused.
            self.parent.kill()
        self.parent.wait()
        if self.parent_fd >= 0:
            os.close(self.parent_fd)
            self.parent_fd = -1
        self.parent = None

    def close(self):
        self.stop_parent()
        if self.writer_fd < 0:
            return
        signal_original(self.writer_fd)
        try:
            os.waitid(os.P_PIDFD, self.writer_fd, os.WEXITED)
        except ChildProcessError:
            # If its original parent already reaped it, pidfd HUP is the fence.
            poller = select.poll()
            poller.register(self.writer_fd, select.POLLIN)
            assert any(events & select.POLLHUP for _, events in poller.poll(0))
        os.close(self.writer_fd)
        self.writer_fd = -1


def namespace(binary):
    assert os.path.samefile("/proc/1/exe", binary), "private test PID namespace required"
    library = ctypes.CDLL(None, use_errno=True)
    library.prctl.argtypes = [ctypes.c_int] + [ctypes.c_ulong] * 4
    library.prctl.restype = ctypes.c_int
    assert library.prctl(36, 1, 0, 0, 0) == 0  # PR_SET_CHILD_SUBREAPER.


def device(stack, root, label):
    master, slave = pty.openpty()
    stack.callback(os.close, master)
    stack.callback(os.close, slave)
    tty.setraw(slave)
    path = os.ttyname(slave)
    identity = os.fstat(slave).st_rdev
    name = root / "sys" / f"{os.major(identity)}:{os.minor(identity)}" / "name"
    name.parent.mkdir(parents=True)
    name.write_text(label)
    return master, slave, path


def listener(stack, path):
    endpoint = stack.enter_context(socket.socket(socket.AF_UNIX, socket.SOCK_STREAM))
    endpoint.bind(str(path))
    endpoint.listen(1)
    endpoint.settimeout(5)
    return endpoint


def receive_descriptor(endpoint):
    item = array.array("i").itemsize
    payload, controls, flags, _ = endpoint.recvmsg(1, socket.CMSG_SPACE(item), socket.MSG_CMSG_CLOEXEC)
    with contextlib.ExitStack() as pending:
        descriptors = []
        for level, kind, data in controls:
            if (level, kind) == (socket.SOL_SOCKET, socket.SCM_RIGHTS):
                values = array.array("i")
                values.frombytes(data[:len(data) // item * item])
                for fd in values:
                    pending.callback(os.close, fd)
                    descriptors.append(fd)
        assert not flags & (socket.MSG_TRUNC | socket.MSG_CTRUNC)
        assert payload == b"R" and len(descriptors) == 1
        pending.pop_all()  # Transfer one owned descriptor to the caller.
        return descriptors[0]



def proc_field(text, name):
    return int(next(line.split()[1] for line in text.splitlines() if line.startswith(name + ":")))


def original_writer(endpoint, parent):
    fd = receive_descriptor(endpoint)
    try:
        peer, uid, _gid = struct.unpack("3i", endpoint.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
        assert uid == os.geteuid() and peer > 0
        if peer != parent:
            assert proc_field(Path(f"/proc/{peer}/status").read_text(), "PPid") == parent
        assert proc_field(Path(f"/proc/self/fdinfo/{fd}").read_text(), "Pid") == peer
        poller = select.poll()
        poller.register(fd, select.POLLIN)
        assert not poller.poll(0)
        return fd
    except BaseException:
        os.close(fd)
        raise


def reusable(fd):
    try:
        fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        return False
    fcntl.flock(fd, fcntl.LOCK_UN)
    return True


def writer(arguments):
    target = arguments[-1]
    with contextlib.ExitStack() as owned:
        if target == "pipe:1":
            output = owned.enter_context(os.fdopen(os.dup(1), "wb", buffering=0))
        else:
            output = owned.enter_context(open(target, "wb", buffering=0))
        endpoint = owned.enter_context(socket.socket(socket.AF_UNIX, socket.SOCK_STREAM))
        endpoint.connect(str(Path(os.environ["BLENT_T1137_ROOT"]) / "writer.sock"))
        fd = os.pidfd_open(os.getpid())
        try:
            endpoint.sendmsg([b"R"], [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array("i", [fd]))])
        finally:
            os.close(fd)
        assert endpoint.recv(1) == b"W"
        assert output.write(b"pixels") == 6
        endpoint.sendall(b"W")
        endpoint.recv(1)  # Remain an actual writer until the supervisor settles it.


def mount(source, target):
    subprocess.run(["mount", "--bind", str(source), str(target)], check=True)


def spawn_creator(group, binary, root, front):
    environment = dict(os.environ, BLENT_T1137_ROOT=str(root), BLENT_T1137_FRONT=front)
    group.parent = subprocess.Popen([binary, "--exact", "camera::native_tests::ownership::t1137_creator_process", "--nocapture"], env=environment, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL)
    group.parent_fd = os.pidfd_open(group.parent.pid)


def create_writer_script(root):
    script = root / "ffmpeg"
    script.write_text("#!/usr/bin/python3\nimport runpy,sys\nrunpy.run_path(" + repr(str(Path(__file__).resolve())) + ")[\"writer\"](sys.argv[1:])\n")
    script.chmod(0o700)


def prefix(fd):
    received = bytearray()
    while len(received) < 6:
        ready, _, _ = select.select([fd], [], [], 2)
        if not ready:
            return False
        part = os.read(fd, 6 - len(received))
        if not part:
            return False
        received.extend(part)
    return received == b"pixels"


def observe(case, group, endpoint, original, foreign):
    if case == "death":
        group.stop_parent()
    held = not reusable(original[1])
    poller = select.poll()
    poller.register(group.writer_fd, select.POLLIN)
    alive = not poller.poll(0)
    delivered = False
    if alive:
        endpoint.sendall(b"W")
        ready, _, _ = select.select([original[0], foreign[0]], [], [], 2)
        delivered = original[0] in ready and prefix(original[0])
    else:
        ready, _, _ = select.select([original[0], foreign[0]], [], [], 0)
    return dict(case=case, original_lease_held=held, writer_alive=alive,
                original_target=delivered, foreign_target=foreign[0] in ready,
                data_after_dead_writer=not alive and bool(ready))


def exercise(case, binary, root):
    with contextlib.ExitStack() as owned:
        original = device(owned, root, "Blent Front")
        foreign = device(owned, root, "Foreign")
        mount(root / "sys", "/sys/dev/char")
        creator = listener(owned, root / "creator.sock")
        receiving = listener(owned, root / "writer.sock")
        create_writer_script(root)
        group = Processes()
        try:
            spawn_creator(group, binary, root, original[2])
            control = owned.enter_context(creator.accept()[0])
            control.settimeout(5)
            assert control.recv(1) == b"L"
            assert not reusable(original[1])
            if case == "replacement":
                mount(foreign[2], original[2])
            control.sendall(b"S")
            endpoint = owned.enter_context(receiving.accept()[0])
            endpoint.settimeout(5)
            group.writer_fd = original_writer(endpoint, group.parent.pid)
            assert control.recv(1) == b"P"
            result = observe(case, group, endpoint, original, foreign)
        finally:
            group.close()
        result["released_after_actual_cleanup"] = reusable(original[1])
    return result



def protocol_case(payload, copies):
    with tempfile.TemporaryDirectory(prefix="blent-t1137-rights-") as directory:
        with contextlib.ExitStack() as owned:
            path = Path(directory) / "lease"
            source = owned.enter_context(open(path, "w+b"))
            probe = owned.enter_context(open(path, "r+b"))
            fcntl.flock(source, fcntl.LOCK_EX)
            sending, receiving = socket.socketpair()
            owned.enter_context(sending)
            owned.enter_context(receiving)
            rights = [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array("i", [source.fileno()] * copies))] if copies else []
            sending.sendmsg([payload], rights)
            source.close()  # Only queued/received descriptions can retain this lease.
            received = None
            try:
                received = receive_descriptor(receiving)
            except AssertionError:
                pass
            accepted = received is not None
            held = True
            if accepted:
                with contextlib.ExitStack() as active:
                    active.callback(os.close, received)
                    held = not reusable(probe)
            released = reusable(probe)
        assert accepted == (payload == b"R" and copies == 1) and held and released


def protocol_tests():
    for payload in [b"R", b"X", b"\0"]:
        for copies in range(9):
            protocol_case(payload, copies)
    print("T1137: 27 bounded native rights cases passed")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--case", choices=["death", "replacement"])
    parser.add_argument("--binary")
    parser.add_argument("--protocol-tests", action="store_true")
    options = parser.parse_args()
    if options.protocol_tests:
        protocol_tests()
        return
    assert options.case is not None and options.binary is not None
    namespace(options.binary)
    with tempfile.TemporaryDirectory(prefix="blent-t1137-") as directory:
        result = exercise(options.case, options.binary, Path(directory))
    print(json.dumps(result), flush=True)
    assert result["case"] == "death" or result["writer_alive"], "T1137 replacement writer missing"
    assert not result["writer_alive"] or (result["original_lease_held"] and result["original_target"]), "T1137 live original output ownership escaped"
    assert not result["foreign_target"] and not result["data_after_dead_writer"] and result["released_after_actual_cleanup"], "T1137 original output retirement escaped"


if __name__ == "__main__":
    main()
