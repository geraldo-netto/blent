"""T1461 synthetic SDR probe; never opens a camera or output device."""
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[4]


def production_filter():
    source = (ROOT / "host/src/camera/decoder.rs").read_text()
    literal = re.search(
        r'format!\("([^"\n]+force_original_aspect_ratio[^"\n]+)"', source
    ).group(1)
    return literal.replace("{turn}", "").replace("{mirror}", "").format(
        160, 120, 160, 120
    )


def tagged_input():
    return subprocess.run(
        ["ffmpeg", "-v", "error", "-f", "lavfi", "-i",
         "color=0x4080c0:s=160x120:r=30", "-vf",
         "scale=in_color_matrix=bt601:out_color_matrix=bt709",
         "-frames:v", "1", "-pix_fmt", "yuv420p", "-c:v", "libx264",
         "-qp", "0", "-threads", "1", "-colorspace", "bt709",
         "-color_primaries", "bt709", "-color_trc", "bt709",
         "-color_range", "tv", "-f", "h264", "pipe:1"],
        capture_output=True, check=True,
    ).stdout


def conversion(encoded, filters):
    result = subprocess.run(
        ["ffmpeg", "-hide_banner", "-loglevel", "info", "-threads", "1",
         "-f", "h264", "-i", "pipe:0", "-vf", filters + ",showinfo",
         "-frames:v", "1", "-pix_fmt", "yuv420p", "-f", "rawvideo", "pipe:1"],
        input=encoded, capture_output=True, check=True,
    )
    assert len(result.stdout) == 28800
    return {
        "filters": filters,
        "yuv": [result.stdout[i] for i in [0, 19200, 24000]],
        "metadata": [line for line in result.stderr.decode().splitlines()
                     if "color_primaries" in line],
    }


def main():
    encoded = tagged_input()
    rows = {
        "production": conversion(encoded, production_filter()),
        "complete_smpte170m_conversion": conversion(
            encoded, "colorspace=all=smpte170m:range=tv:format=yuv420p"
        ),
    }
    output = json.dumps(rows, indent=2) + "\n"
    Path(__file__).with_name("color.json").write_text(output)
    print(output, end="")


if __name__ == "__main__":
    main()
