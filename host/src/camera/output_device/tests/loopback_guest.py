#!/usr/bin/env python3
"""Offline fixture. Run owns the foreground VM; check follows its actual exit."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess

SUITE = "camera::output_device::tests::virtual_driver::t1143_virtual_output_process_fixture"


def fingerprint(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def copy(source, target):
    before = fingerprint(source)
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, target)
    assert before == fingerprint(source) == fingerprint(target), "input changed during copy"


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_subparsers(dest="mode", required=True)
    run = modes.add_parser("run")
    for name in ["kernel", "busybox", "binary", "output"]:
        run.add_argument("--" + name, type=Path, required=True)
    run.add_argument("--release", required=True)
    run.add_argument("--suite", default=SUITE)
    run.add_argument("--ffmpeg", type=Path)
    run.add_argument("--camera-pair", action="store_true")
    modes.add_parser("check").add_argument("output", type=Path)
    options = parser.parse_args()
    options.output = options.output.resolve()
    if options.mode == "run":
        validate_arguments(parser, options)
    return options


def validate_arguments(parser, options):
    if not re.fullmatch(r"[0-9][A-Za-z0-9_.+-]{0,127}", options.release):
        parser.error("invalid installed module release")
    if not re.fullmatch(r"[A-Za-z0-9_:]{1,240}", options.suite):
        parser.error("invalid native fixture name")
    if "," in str(options.output):
        parser.error("output path cannot contain a QEMU option delimiter")


def loopback_options(pair):
    if pair:
        return " devices=2 video_nr=20,21 'card_label=\"Blent Front,Blent Rear\"' exclusive_caps=1,1"
    return " devices=1 video_nr=20 'card_label=\"Blent Front\"' exclusive_caps=1"


def modules(release, root, pair):
    paths = {}
    for name in ["9pnet_virtio", "9p", "v4l2loopback"]:
        output = subprocess.check_output(["modprobe", "--show-depends", "-S", release, name], text=True)
        for line in output.splitlines():
            if line.startswith("insmod "):
                paths.setdefault(Path(line.split()[1]), None)
    commands = []
    for source in paths:
        target = root / "modules" / source.name.removesuffix(".zst")
        with target.open("wb") as output:
            subprocess.run(["zstd", "-dq", "-c", str(source)], stdout=output, check=True)
        command = "insmod /modules/" + shlex.quote(target.name)
        if target.name == "v4l2loopback.ko":
            command += loopback_options(pair)
        commands.append(command)
    return commands


def libraries(executable, root):
    output = subprocess.check_output(["ldd", str(executable)], text=True)
    for name in re.findall(r"(?:=>\s+|^\s*)(/\S+)", output, re.M):
        copy(Path(name), root / name.lstrip("/"))


def programs(options, stage):
    copy(options.binary, stage / "bin/blent-tests")
    oracle = stage / "bin/v4l2-oracle"
    subprocess.run(["cc", "-std=c11", "-D_GNU_SOURCE", "-Wall", "-Wextra", "-Werror",
                    str(Path(__file__).with_name("abi.c")), "-o", str(oracle)], check=True)
    binaries = [stage / "bin/blent-tests", oracle]
    if options.ffmpeg is not None:
        target = stage / "bin/ffmpeg"
        copy(options.ffmpeg, target)
        binaries.append(target)
    for executable in binaries:
        libraries(executable, stage / "root")
    for directory in ["lib", "lib64", "usr/lib"]:
        (stage / "root" / directory).mkdir(parents=True, exist_ok=True)


def init_script(commands, suite):
    return """#!/bin/sh
set -eu
/bin/busybox --install -s /bin
mount -t proc proc /proc
mount -t sysfs sys /sys
mount -t devtmpfs dev /dev
""" + "\n".join(commands) + """
mount -t 9p -o trans=virtio,version=9p2000.L,ro blent-bin /work
mount -t 9p -o trans=virtio,version=9p2000.L blent-evidence /evidence
mount --bind /work/root/lib /lib
mount --bind /work/root/lib64 /lib64
mount --bind /work/root/usr/lib /usr/lib
export LLVM_PROFILE_FILE=/evidence/v4l2-%m-%p.profraw
export BLENT_T1143_DEVICE=/dev/video20
export BLENT_T1137_REAR=/dev/video21
export BLENT_T1143_ORACLE=/work/bin/v4l2-oracle
export BLENT_T1144_FFMPEG=/work/bin/ffmpeg
echo T1143_DRIVER_READY
if /work/bin/blent-tests --exact """ + shlex.quote(suite) + """ --nocapture; then
    result=0
else
    result=$?
fi
echo "$result" > /evidence/result
rmmod v4l2loopback
echo T1143_MODULE_RETIRED
sync
echo T1143_VM_DONE
poweroff -f
"""


def archive(root, destination):
    names = subprocess.check_output(["find", ".", "-print0"], cwd=root)
    with destination.open("wb") as output:
        subprocess.run(["cpio", "--null", "-o", "--format=newc", "--quiet"],
                       cwd=root, input=names, stdout=output, check=True)


def prepare(options):
    options.output.mkdir(parents=True, exist_ok=False)
    initrd = options.output / "initrd"
    for directory in ["bin", "proc", "sys", "dev", "tmp", "modules", "work", "evidence", "lib", "lib64", "usr/lib"]:
        (initrd / directory).mkdir(parents=True, exist_ok=True)
    copy(options.busybox, initrd / "bin/busybox")
    (initrd / "bin/sh").symlink_to("busybox")
    copy(options.kernel, options.output / "vmlinuz")
    commands = modules(options.release, initrd, options.camera_pair)
    programs(options, options.output / "stage")
    script = initrd / "init"
    script.write_text(init_script(commands, options.suite))
    script.chmod(0o755)
    archive(initrd, options.output / "initramfs.cpio")
    (options.output / "evidence").mkdir()


def qemu(root):
    return ["qemu-system-x86_64", "-enable-kvm", "-cpu", "host", "-smp", "2", "-m", "2048",
            "-nodefaults", "-no-user-config", "-nographic", "-serial", "stdio", "-monitor", "none",
            "-net", "none", "-no-reboot", "-kernel", str(root / "vmlinuz"), "-initrd", str(root / "initramfs.cpio"),
            "-append", "console=ttyS0 rdinit=/init panic=-1",
            "-fsdev", f"local,id=bin,path={root / 'stage'},security_model=none,readonly=on",
            "-device", "virtio-9p-pci,fsdev=bin,mount_tag=blent-bin",
            "-fsdev", f"local,id=evidence,path={root / 'evidence'},security_model=none",
            "-device", "virtio-9p-pci,fsdev=evidence,mount_tag=blent-evidence"]


def check(root):
    log = (root / "guest.log").read_text()
    assert (root / "evidence/result").read_text().strip() == "0"
    for marker in ["T1143_DRIVER_READY", "T1143_MODULE_RETIRED", "T1143_VM_DONE"]:
        assert marker in log, "guest stage missing: " + marker
    assert not re.search(r"BUG:|Oops:|Kernel panic", log), "native kernel failure"
    print("Native fixture passed; module retired; evidence:", root)


def launch(options):
    prepare(options)
    command = qemu(options.output)
    metadata = {"command": command, "release": options.release,
                "kernel_sha256": fingerprint(options.output / "vmlinuz"),
                "test_binary_sha256": fingerprint(options.output / "stage/bin/blent-tests"),
                "oracle_source_sha256": fingerprint(Path(__file__).with_name("abi.c")),
                "runner_sha256": fingerprint(Path(__file__))}
    (options.output / "run.json").write_text(json.dumps(metadata, indent=2) + "\n")
    print("Foreground VM; log:", options.output / "guest.log", flush=True)
    with (options.output / "guest.log").open("w") as output:
        os.dup2(output.fileno(), 1)
        os.dup2(output.fileno(), 2)
        # Replace this owner rather than leave a second controller/process to
        # lose. The caller waits for this same foreground identity before check.
        os.execvp(command[0], command)


def main():
    options = arguments()
    if options.mode == "check":
        check(options.output)
    else:
        launch(options)


if __name__ == "__main__":
    main()
