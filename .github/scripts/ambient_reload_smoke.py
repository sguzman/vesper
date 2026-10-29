"""QOL06 external manifest hot reload: re-scale, add, remove, reject invalid."""
from __future__ import annotations
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
ORIGINAL = ROOT / "examples/ambient-showcase"
PLAYER = ROOT / "target/debug/vesper-player"
OUT = Path("/tmp/vesper-ambient-reload-qa")

def call(*args, check=True):
    return subprocess.run(args, capture_output=True, text=True, check=check, timeout=30)

def wait(log, phrase, prior=0):
    deadline=time.monotonic()+25
    while time.monotonic()<deadline:
        if log.exists() and log.read_text(errors="replace").count(phrase)>prior:
            return
        time.sleep(.15)
    raise AssertionError(f"missing {phrase}: {log.read_text(errors='replace')[-2000:]}")

def capture(win, path):
    time.sleep(.55)
    call("import", "-window", win, str(path))
    assert call("identify","-format","%w %h",str(path)).stdout=="1280 720"
    return path

def crop(path, new):
    call("convert",str(path),"-crop","410x200+710+110","+repage",str(new))
    return new

def difference(first,second):
    result=call("compare","-metric","AE",str(first),str(second),"null:",check=False)
    m=re.match(r"^\d+",result.stderr.strip())
    assert m,result.stderr
    return int(m.group())

def main():
    OUT.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="vesper-ambient-reload-") as tmp:
        root=Path(tmp)
        game=root/"game"
        shutil.copytree(ORIGINAL,game)
        settings=root/"prefs"
        settings.mkdir()
        (settings/"player-preferences.json").write_text(
            json.dumps({"format_version":1,"reveal_speed":"instant"}))
        env=dict(os.environ,LIBGL_ALWAYS_SOFTWARE="1",
                 VESPER_TRACE_SCENES="1",
                 VESPER_PREFERENCES_DIR=str(settings),
                 VESPER_SAVE_DIR=str(root/"saves"))
        log=OUT/"reload.log"
        with log.open("w") as stream:
            proc=subprocess.Popen([str(PLAYER),str(game),"--watch"],env=env,
                                  stdout=stream,stderr=subprocess.STDOUT)
            try:
                win=None
                deadline=time.monotonic()+25
                while time.monotonic()<deadline:
                    result=call("xdotool","search","--name","Morning at the Perch",check=False)
                    if result.stdout.strip():
                        win=result.stdout.splitlines()[0]; break
                    time.sleep(.15)
                assert win,"ambient hot reload player window missing"
                call("xdotool","windowfocus","--sync",win)
                wait(log,"Vesper ambient actors spawned: 1")
                wait(log,"Vesper scene dialogue: dawn")
                time.sleep(1.0)
                baseline=crop(capture(win,OUT/"initial.png"),OUT/"initial-crop.png")
                manifest=game/"game.toml"
                original=manifest.read_text()
                assert "scale = 2.0" in original
                edited=original.replace("scale = 2.0","scale = 1.0")
                applied=log.read_text(errors="replace").count("Vesper hot reload applied")
                manifest.write_text(edited)
                wait(log,"Vesper hot reload applied",applied)
                scaled=crop(capture(win,OUT/"scaled.png"),OUT/"scaled-crop.png")
                assert difference(baseline,scaled)>150,"valid edit did not re-scale bird"

                addition='\n[ambient_actors.sparrow]\nimage = "assets/bird.png"\nanchor = [0.58, 0.28]\nscale = 1.3\nscene = "dawn"\n'
                manifest.write_text(edited+addition)
                wait(log,"Vesper ambient actor reconciled new entity: sparrow")
                added=crop(capture(win,OUT/"added.png"),OUT/"added-crop.png")
                assert difference(scaled,added)>150,"new ambient actor not visible"

                manifest.write_text(edited)
                wait(log,"Vesper ambient actor reconciled removed entity: sparrow")
                removed=crop(capture(win,OUT/"removed.png"),OUT/"removed-crop.png")
                assert difference(scaled,removed)<500,"removed actor pixels persisted"

                invalid=edited.replace('scene = "dawn"','scene = "missing"')
                manifest.write_text(invalid)
                wait(log,"Vesper hot reload rejected")
                rejected=crop(capture(win,OUT/"rejected.png"),OUT/"rejected-crop.png")
                assert difference(removed,rejected)<500,"invalid reload disturbed live sprites"
                assert proc.poll() is None,"hot reload crashed the player"
                print("QOL06: update, add, remove, reject, preserve runtime PASS",flush=True)
            finally:
                if proc.poll() is None:
                    proc.terminate()
                    proc.wait(timeout=10)
if __name__=="__main__": main()
