"""QOL07: three independent moving ambient sprites in the real Bevy player."""
from __future__ import annotations
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
GAME = ROOT / "examples/moving-meadow"
PLAYER = ROOT / "target/debug/vesper-player"
OUT = Path("/tmp/vesper-ambient-motion-qa")
SIZES = ((946, 511), (955, 520), (800, 600), (900, 700),
         (1280, 720), (1920, 1080))

def call(*args, check=True):
    return subprocess.run(args, text=True, capture_output=True,
                          check=check, timeout=30)

def wait(log, needle):
    deadline=time.monotonic()+25
    while time.monotonic()<deadline:
        if needle in log.read_text(errors="replace"):
            return
        time.sleep(.15)
    raise AssertionError(f"missing {needle}: {log.read_text(errors='replace')[-1400:]}")

def shot(win, where, width=1280, height=720):
    limit=time.monotonic()+15
    while time.monotonic()<limit:
        time.sleep(.25)
        call("import", "-window", win, str(where))
        got=call("identify","-format","%w %h",str(where)).stdout
        assert got==f"{width} {height}",(got,width,height)
        # A Bevy scene can expose UI text before its asynchronous background
        # texture has drawn. Require a real multi-color backdrop above the UI.
        # Otherwise reduced-motion comparisons mistake asset arrival for motion.
        top = call("convert", str(where),
                   "-crop", f"{width//2}x{max(60,height//4)}+{width//4}+0",
                   "+repage", "-format", "%k", "info:").stdout
        if int(top) > 10:
            return where
    raise AssertionError(f"unrendered ambient motion frame: {where}")

def crop(src,dst,region):
    call("convert",str(src),"-crop",region,"+repage",str(dst))
    return dst

def difference(a,b):
    r=call("compare","-metric","AE",str(a),str(b),"null:",check=False)
    m=re.match(r"^\d+",r.stderr.strip())
    assert m,r.stderr
    return int(m.group())

def scenario(mode,reduced=False):
    folder=OUT/mode
    folder.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="vesper-moving-meadow-") as temp:
        root=Path(temp)
        prefs=root/"prefs"
        prefs.mkdir()
        (prefs/"player-preferences.json").write_text(json.dumps({
            "format_version":1, "reveal_speed":"instant",
            "reduced_motion":reduced,
        }))
        env=dict(os.environ,LIBGL_ALWAYS_SOFTWARE="1",
                 VESPER_PREFERENCES_DIR=str(prefs),
                 VESPER_SAVE_DIR=str(root/"saves"))
        log=folder/"player.log"
        with log.open("w") as stream:
            proc=subprocess.Popen([str(PLAYER),str(GAME)],env=env,
                                  stdout=stream,stderr=subprocess.STDOUT)
            try:
                win=None
                deadline=time.monotonic()+25
                while time.monotonic()<deadline:
                    result=call("xdotool","search","--name","The Moving Meadow",check=False)
                    if result.stdout.strip():
                        win=result.stdout.splitlines()[0]
                        break
                    time.sleep(.2)
                assert win,"moving meadow window missing"
                call("xdotool","windowfocus","--sync",win)
                wait(log,"Vesper ambient actors spawned: 3")
                time.sleep(.8)
                first=shot(win,folder/"start.png")
                if not reduced:
                    # The pause overlay must freeze every ambient ECS clock.
                    # Compare two frames with the same overlay, not a frame
                    # against the unpaused UI.
                    call("xdotool", "key", "--clearmodifiers", "p")
                    wait(log, "Vesper story paused")
                    pause_a=shot(win,folder/"paused-start.png")
                    time.sleep(.9)
                    pause_b=shot(win,folder/"paused-later.png")
                    assert difference(pause_a,pause_b)<40, (
                        "ambient animation changed while story was paused")
                    call("xdotool", "key", "--clearmodifiers", "p")
                    wait(log, "Vesper story resumed")
                time.sleep(1.9)
                middle=shot(win,folder/"middle.png")
                if not reduced:
                    # Save while the plane is in mid-flight. The saved clock is
                    # authoritative and proves the ECS phase was snapshotted.
                    call("xdotool", "key", "--clearmodifiers", "F5")
                    wait(log, "Vesper save written")
                    files = list((root / "saves").rglob("*.json"))
                    assert len(files) == 1, f"missing quicksave: {files}"
                    saved = json.loads(files[0].read_text())
                    phases = saved["presentation"]["ambient_motion_elapsed"]
                    assert set(phases) == {"bird", "walker", "plane"}, phases
                    assert 0.0 < phases["plane"] < 5.5, phases
                    assert all(0.0 <= value <= 240.0 for value in phases.values())
                time.sleep(2.1)
                last=shot(win,folder/"end.png")
                if not reduced:
                    regions={
                        "plane":"1280x160+0+0",
                        "bird":"1280x250+0+145",
                        "walker":"1280x280+0+420",
                    }
                    for label,rect in regions.items():
                        a=crop(first,folder/f"{label}-start.png",rect)
                        b=crop(middle,folder/f"{label}-mid.png",rect)
                        delta=difference(a,b)
                        assert delta>60,f"{label} stayed stationary: {delta}"
                        print(f"QOL07 moving {label}: pixel delta {delta}",flush=True)
                    # The one-shot plane reaches its terminal point; the
                    # characters can keep moving and do not affect it.
                    assert difference(
                        crop(middle,folder/"plane-mid2.png",regions["plane"]),
                        crop(last,folder/"plane-end.png",regions["plane"]))>40
                    call("xdotool", "key", "--clearmodifiers", "F9")
                    wait(log, "Vesper save restored")
                    wait(log, "Vesper ambient motion clock restored: plane to")
                    restored_match = re.search(
                        r"Vesper ambient motion clock restored: plane to ([0-9.]+)s",
                        log.read_text(errors="replace"))
                    assert restored_match is not None, "loaded plane clock was not applied"
                    assert abs(float(restored_match.group(1)) - phases["plane"]) < .005
                    loaded=shot(win,folder/"after-quickload.png")
                    loaded_plane=crop(
                        loaded, folder/"plane-after-quickload.png", regions["plane"])
                    assert difference(loaded_plane,folder/"plane-end.png") > 60, (
                        "quickload did not move the plane back to its saved phase")
                    for width,height in SIZES:
                        call("xdotool","windowsize","--sync",win,str(width),str(height))
                        shot(win,folder/f"{width}x{height}.png",width,height)
                else:
                    assert difference(first,middle)<40, (
                        "reduced motion did not freeze authored actors")
                    assert difference(middle,last)<40, (
                        "reduced motion unexpectedly advanced ambient actors")
                assert proc.poll() is None,"moving meadow player crashed"
            finally:
                if proc.poll() is None:
                    proc.terminate()
                    proc.wait(timeout=10)

def main():
    OUT.mkdir(parents=True,exist_ok=True)
    scenario("moving")
    scenario("reduced-motion",reduced=True)
    print("QOL07 moving actors, three paths, pause, mid-flight save, six sizes and reduced motion PASS",flush=True)

if __name__=="__main__": main()
