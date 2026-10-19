//! T1115: real C admission precedes Rust publication, on the original FIFO fd.
use super::*;
use anyhow::{ensure, Context};
use std::{os::fd::AsRawFd, process::Stdio};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::{ChildStdin, ChildStdout, Command},
};

fn build(root: &Path) -> Result<PathBuf> {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let binary = root.join("idle-probe");
    let output = std::process::Command::new("cc")
        .args([
            "-std=c11",
            "-O1",
            "-g",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-fsanitize=address,undefined",
            "-fno-pie",
            "-no-pie",
            "-I",
        ])
        .arg(project.join("evdi"))
        .arg(project.join("tests/idle_request_probe.c"))
        .arg(project.join("evdi/idle_control.c"))
        .arg("-o")
        .arg(&binary)
        .output()?;
    ensure!(
        output.status.success(),
        "T1115 native probe compile: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(binary)
}

fn inherit(fd: i32) -> std::io::Result<()> {
    if unsafe { libc::fcntl(fd, libc::F_SETFD, 0) } < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

async fn line(output: &mut ChildStdout) -> Result<Vec<u8>> {
    let mut reply = Vec::with_capacity(8);
    for _ in 0..8 {
        let byte = output.read_u8().await?;
        reply.push(byte);
        if byte == b'\n' {
            return Ok(reply);
        }
    }
    anyhow::bail!("T1115 oversized native reply")
}

async fn exchange(lease: &Lease, input: &mut ChildStdin, output: &mut ChildStdout) -> Result<()> {
    ensure!(
        line(output).await? == b"READY\n",
        "T1115 native admission missing"
    );
    for (interval, expected) in [(500, b"500\n".as_slice()), (1000, b"1000\n"), (200, b"0\n")] {
        lease.publish(interval)?;
        input.write_all(b"?").await?;
        ensure!(
            line(output).await? == expected,
            "T1115 native cadence differs from Rust publication {interval}"
        );
    }
    Ok(())
}

fn source_command(binary: &Path, lease: &Lease) -> Result<Command> {
    // Reserve above stdio before fork; never overwrite an exec-error pipe or
    // depend on the parent's standard descriptors being open.
    let descriptor = unsafe { libc::fcntl(lease._inode.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 3) };
    let original_source = unsafe { blent_config::linux::packet::owned(descriptor) }?;
    let mut command = Command::new(binary);
    command
        .arg(&lease.path)
        .arg(original_source.as_raw_fd().to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    unsafe {
        command.pre_exec(move || inherit(original_source.as_raw_fd()));
    }
    Ok(command)
}

async fn native_flow(
    binary: &Path,
    lease: &Lease,
    scope: &blent::native_scope::Scope,
) -> Result<()> {
    let mut command = source_command(binary, lease)?;
    let (children, retired) = scope.group();
    let mut child = children.spawn(&mut command)?;
    drop(command); // Parent duplicate ends only after actual child handoff.
    let mut input = child.take_stdin().context("T1115 original native input")?;
    let mut output = child
        .take_stdout()
        .context("T1115 original native output")?;
    let exchanged = tokio::time::timeout(
        Duration::from_secs(5),
        exchange(lease, &mut input, &mut output),
    )
    .await;
    drop(input);
    drop(output);
    let status = child.finish(Duration::from_secs(5)).await;
    drop(children);
    // Deadline/errors never substitute for the original process's real fence.
    while !retired.is_complete() {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    exchanged??;
    ensure!(status?.success(), "T1115 native probe failed");
    Ok(())
}

#[tokio::test]
async fn t1115_real_native_reader_accepts_only_original_rust_source_publications() {
    let runtime = tokio::runtime::Handle::current();
    let (scope, retired) = blent::native_scope::Scope::default().group();
    let result = scope
        .blocking(move |scope| {
            let root = super::lease_tests::private_root();
            let binary = build(root.path())?;
            let lease = super::tests::lease(root.path());
            runtime.block_on(native_flow(&binary, &lease, &scope))
        })
        .await;
    drop(scope);
    super::tests::native_until(|| retired.is_complete()).await;
    result.unwrap().unwrap();
}

async fn rejected_arguments(binary: &Path, scope: &blent::native_scope::Scope) -> Result<()> {
    for argument in [
        "",
        " ",
        "-1",
        "+3",
        "0",
        "1",
        "2",
        "3x",
        "NaN",
        "2147483647",
        "2147483648",
        "9223372036854775808",
    ] {
        let mut command = Command::new(binary);
        command.arg("/unused").arg(argument);
        let output = scope
            .output(&mut command, None, Duration::from_secs(5))
            .await?;
        ensure!(
            output.status.code() == Some(2),
            "T1115 invalid descriptor {argument:?}"
        );
        ensure!(
            output.stdout.is_empty(),
            "T1115 rejected descriptor produced reply"
        );
    }
    for count in [0, 1, 3] {
        let mut command = Command::new(binary);
        command.args(std::iter::repeat_n("invalid", count));
        let output = scope
            .output(&mut command, None, Duration::from_secs(5))
            .await?;
        ensure!(
            output.status.code() == Some(2),
            "T1115 invalid argument count {count}"
        );
    }
    Ok(())
}

async fn rejected_commands(
    binary: &Path,
    lease: &Lease,
    scope: &blent::native_scope::Scope,
) -> Result<()> {
    for command_byte in 0u8..=255 {
        if command_byte == b'?' {
            continue;
        }
        let mut command = source_command(binary, lease)?;
        let output = scope
            .output(&mut command, Some(&[command_byte]), Duration::from_secs(5))
            .await?;
        ensure!(
            output.status.code() == Some(4),
            "T1115 invalid command {command_byte}"
        );
        ensure!(
            output.stdout == b"READY\n",
            "T1115 invalid command changed reader"
        );
    }
    let mut command = source_command(binary, lease)?;
    let output = scope
        .output(&mut command, Some(&[]), Duration::from_secs(5))
        .await?;
    ensure!(output.status.code() == Some(4), "T1115 source EOF accepted");
    Ok(())
}

#[tokio::test]
async fn t1115_native_probe_rejects_invalid_descriptor_command_and_eof_boundaries() {
    let runtime = tokio::runtime::Handle::current();
    let (scope, retired) = blent::native_scope::Scope::default().group();
    let result = scope
        .blocking(move |scope| {
            let root = super::lease_tests::private_root();
            let binary = build(root.path())?;
            let lease = super::tests::lease(root.path());
            runtime.block_on(async {
                rejected_arguments(&binary, &scope).await?;
                rejected_commands(&binary, &lease, &scope).await
            })
        })
        .await;
    drop(scope);
    super::tests::native_until(|| retired.is_complete()).await;
    result.unwrap().unwrap();
}
