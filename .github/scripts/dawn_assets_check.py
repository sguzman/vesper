"""Validate every original Dawn asset against its committed provenance ledger.

Requires only Python 3.11+ standard library; no art-generation dependencies in
CI, the packaged executable, or Bevy's update/render thread.
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import struct
import tomllib
import wave

ROOT = Path(__file__).resolve().parents[2] / "examples/dawn-post-office"
EXPECTED = {"background": 7, "character_expression": 9,
            "original_synthesized_wav": 7}


def main() -> None:
    ledger = json.loads((ROOT / "assets/manifest.json").read_text())
    manifest = tomllib.loads((ROOT / "game.toml").read_text())
    assert ledger["schema_version"] == 1
    assert ledger["game"] == manifest["id"] == "dawn-post-office"
    assert ledger["production_only"] is True
    assert (ROOT / ledger["generator"]).is_file()
    paths = set()
    counts = {kind: 0 for kind in EXPECTED}
    for entry in ledger["assets"]:
        relative = Path(entry["path"])
        assert relative.parts[0] == "assets" and all(
            part not in ("..", ".") for part in relative.parts), entry
        full = (ROOT / relative).resolve()
        assert full.is_relative_to(ROOT.resolve()), entry
        assert full.is_file(), full
        assert entry["path"] not in paths, entry
        paths.add(entry["path"])
        payload = full.read_bytes()
        assert len(payload) == entry["bytes"], full
        git_sha = hashlib.sha1(
            f"blob {len(payload)}\0".encode() + payload
        ).hexdigest()
        assert git_sha == entry["git_blob_sha1"], full
        kind = entry["kind"]
        assert kind in counts, entry
        counts[kind] += 1
        if kind in ("background", "character_expression"):
            assert payload[:8] == b"\x89PNG\r\n\x1a\n", full
            assert payload[12:16] == b"IHDR", full
            width, height, depth, color_type = struct.unpack(
                ">IIBB", payload[16:26])
            assert [width, height] == entry["size_px"], full
            assert depth == 8, (full, depth)
            if kind == "character_expression":
                assert color_type == 6, (full, color_type)
                assert entry["alpha"] == "transparent_rgba", full
        else:
            with wave.open(str(full), "rb") as w:
                assert w.getnchannels() == entry["channels"] == 1
                assert w.getsampwidth() * 8 == entry["pcm_bits"] == 16
                assert w.getframerate() == entry["sample_rate_hz"] == 16000
                assert w.getnframes() > 1000
    assert counts == EXPECTED, counts

    referenced = set(manifest["backgrounds"].values())
    referenced |= set(manifest["music"].values())
    referenced |= set(manifest["sounds"].values())
    for actor in manifest["characters"].values():
        referenced |= set(actor["expressions"].values())
    assert referenced == paths, {
        "unregistered": sorted(referenced - paths),
        "unused": sorted(paths - referenced),
    }
    print(f"Verified {len(paths)} original assets, sizes, PNG headers, "
          f"PCM format, Git blob hashes, and game manifest references")


if __name__ == "__main__":
    main()
