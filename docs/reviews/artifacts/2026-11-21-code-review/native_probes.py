"""Run from the repository root; all generated data belongs to an external directory."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile


def runtime_path(output):
    with tempfile.TemporaryDirectory(prefix='blent-review-') as directory:
        root = Path(directory)
        runtime = root / ('r' * 100)
        runtime.mkdir(mode=0o700)
        env = dict(os.environ, HOME=str(root), XDG_RUNTIME_DIR=str(runtime),
                   XDG_CONFIG_HOME=str(root / 'config'), DISPLAY='', WAYLAND_DISPLAY='',
                   DBUS_SESSION_BUS_ADDRESS='unix:path=/nonexistent')
        result = subprocess.run([str(Path('target/debug/blent-gui').resolve())],
                                env=env, capture_output=True, text=True)
        data = dict(probe='long private runtime path', runtime_path_bytes=len(os.fsencode(runtime)),
                    exit=result.returncode, stdout=result.stdout, stderr=result.stderr)
        (output / 'long-runtime.json').write_text(json.dumps(data, indent=2) + '\n')


def encode(matrix):
    return subprocess.run([
        'ffmpeg', '-v', 'error', '-f', 'lavfi', '-i', 'color=red:s=160x120:r=30',
        '-vf', 'scale=in_color_matrix=bt601:out_color_matrix=' + matrix,
        '-frames:v', '1', '-pix_fmt', 'yuv420p', '-c:v', 'libx264', '-qp', '0',
        '-threads', '1', '-colorspace', 'smpte170m' if matrix == 'bt601' else 'bt709',
        '-f', 'h264', 'pipe:1'], check=True, capture_output=True).stdout


def decode(encoded):
    filters = ('scale=160:120:force_original_aspect_ratio=decrease,'
               'pad=160:120:(ow-iw)/2:(oh-ih)/2,format=yuv420p')
    return subprocess.run([
        'ffmpeg', '-v', 'error', '-threads', '1', '-f', 'h264', '-i', 'pipe:0',
        '-vf', filters, '-frames:v', '1', '-pix_fmt', 'yuv420p', '-f', 'rawvideo',
        'pipe:1'], input=encoded, check=True, capture_output=True).stdout


def camera_colors(output):
    rows = []
    for matrix in ['bt601', 'bt709']:
        decoded = decode(encode(matrix))
        y, u, v = decoded[0], decoded[19200], decoded[24000]
        c, d, e = y - 16, u - 128, v - 128
        rgb = [min(255, max(0, (value + 128) >> 8))
               for value in [298*c + 409*e, 298*c - 100*d - 208*e, 298*c + 516*d]]
        rows.append(dict(input_matrix=matrix, decoded_yuv=[y, u, v],
                         preview_rgb_from_actual_integer_formula=rgb))
        (output / (matrix + '.yuv')).write_bytes(decoded)
    (output / 'camera-color.json').write_text(json.dumps(rows, indent=2) + '\n')


def main():
    output = Path(sys.argv[1])
    output.mkdir(parents=True, exist_ok=True)
    runtime_path(output)
    camera_colors(output)


if __name__ == '__main__':
    main()
