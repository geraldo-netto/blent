"""Owned offline compiler/codec processes; callers own their bounded input/logs.

Supported tools keep children in the original process group and join them on
normal exit. Process exit is not proof of an arbitrary native device fence.
"""
import os
import signal
import subprocess


def start(arguments, **options):
    return subprocess.Popen(arguments, start_new_session=True, **options)


def joined(process):
    """Observe original retirement even if cleanup is interrupted again."""
    while True:
        try:
            return process.wait()
        except (KeyboardInterrupt, InterruptedError):
            continue


def stop(process):
    """Keep the unreaped original identity through group stop and actual join."""
    try:
        if process.poll() is None:
            os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass  # Original group ended between observation and signal.
    finally:
        status = joined(process)
    return status


def wait(process):
    try:
        return process.wait()
    except BaseException:
        stop(process)
        raise


def close_pipes(process):
    for stream in (process.stdin, process.stdout, process.stderr):
        if stream is not None:
            stream.close()


def close(process):
    try:
        stop(process)
    finally:
        close_pipes(process)


def logged(arguments, environment, logfile, cwd):
    with logfile.open('w') as output:
        process = start(arguments, cwd=cwd, env=environment,
                        stdin=subprocess.DEVNULL, stdout=output, stderr=subprocess.STDOUT)
        return wait(process)


def captured(arguments, input=None, cwd=None):
    # Diagnostics inherit the caller's log so compilation/analysis progress is
    # observable. Stdout is owned bounded data for the selected offline tool.
    process = start(arguments, cwd=cwd, stdin=subprocess.PIPE if input is not None else subprocess.DEVNULL,
                    stdout=subprocess.PIPE)
    try:
        output, errors = process.communicate(input)
        return subprocess.CompletedProcess(arguments, process.returncode, output, errors)
    except BaseException:
        stop(process)
        raise
    finally:
        close_pipes(process)


def output(arguments, **options):
    result = captured(arguments, **options)
    result.check_returncode()
    return result.stdout
