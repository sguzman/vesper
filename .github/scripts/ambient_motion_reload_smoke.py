"""QOL07: remove/reinsert live motion without keeping a stale flipbook image."""
from __future__ import annotations
import json, os, re, shutil, subprocess, tempfile, time
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
GAME=ROOT/"examples/moving-meadow"
PLAYER=ROOT/"target/debug/vesper-player"
OUT=Path("/tmp/vesper-ambient-motion-reload-qa")

def cmd(*args,check=True):
    return subprocess.run(args,text=True,capture_output=True,check=check,timeout=30)

def wait(log,fragment,previous=0):
    until=time.monotonic()+25
    while time.monotonic()<until:
        if log.exists() and log.read_text(errors="replace").count(fragment)>previous:
            return
        time.sleep(.2)
    raise AssertionError(f"missing '{fragment}' in {log.read_text(errors='replace')[-1600:]}")

def shot(win,path):
    until=time.monotonic()+12
    while time.monotonic()<until:
        time.sleep(.3)
        cmd("import","-window",win,str(path))
        assert cmd("identify","-format","%w %h",str(path)).stdout=="1280 720"
        # Background must be loaded, not only the overlaid dialogue.
        colors=int(cmd("convert",str(path),"-crop","640x180+320+0",
                       "+repage","-format","%k","info:").stdout)
        if colors>10:
            return path
    raise AssertionError("frame never received backdrop pixels")

def crop(a,b):
    cmd("convert",str(a),"-crop","500x220+0+400","+repage",str(b))
    return b

def delta(a,b):
    r=cmd("compare","-metric","AE",str(a),str(b),"null:",check=False)
    m=re.match(r"^\d+",r.stderr.strip())
    assert m,r.stderr
    return int(m.group())

def main():
    OUT.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="vesper-motion-reload-") as tmp:
        root=Path(tmp); game=root/"game"
        shutil.copytree(GAME,game)
        prefs=root/"prefs";prefs.mkdir()
        (prefs/"player-preferences.json").write_text(
            json.dumps({"format_version":1,"reveal_speed":"instant"}))
        env=dict(os.environ,LIBGL_ALWAYS_SOFTWARE="1",
                 VESPER_PREFERENCES_DIR=str(prefs),
                 VESPER_SAVE_DIR=str(root/"saves"))
        log=OUT/"player.log"
        with log.open("w") as stream:
            proc=subprocess.Popen([str(PLAYER),str(game),"--watch"],env=env,
                                  stdout=stream,stderr=subprocess.STDOUT)
            try:
                win=None;until=time.monotonic()+25
                while time.monotonic()<until:
                    found=cmd("xdotool","search","--name","The Moving Meadow",check=False)
                    if found.stdout.strip():
                        win=found.stdout.splitlines()[0];break
                    time.sleep(.2)
                assert win,"moving meadow did not open"
                cmd("xdotool","windowfocus","--sync",win)
                wait(log,"Vesper ambient actors spawned: 3")
                shot(win,OUT/"before.png")
                manifest=game/"game.toml"
                original=manifest.read_text()
                needle="[ambient_actors.walker.motion]\n"
                end="[ambient_actors.bird]\n"
                assert original.count(needle)==1 and original.count(end)==1
                start,rest=original.split(needle,1)
                _,remaining=rest.split(end,1)
                static=start+end+remaining
                manifest.write_text(static)
                wait(log,"Vesper ambient motion reconciled: walker")
                frozen_a=crop(shot(win,OUT/"static-a.png"),OUT/"static-a-crop.png")
                time.sleep(.9)
                frozen_b=crop(shot(win,OUT/"static-b.png"),OUT/"static-b-crop.png")
                assert delta(frozen_a,frozen_b)<40,(
                    "static walker continued moving or retained an active flipbook")

                previous=log.read_text(errors="replace").count(
                    "Vesper ambient motion reconciled: walker")
                manifest.write_text(original)
                wait(log,"Vesper ambient motion reconciled: walker",previous)
                live_a=crop(shot(win,OUT/"moving-a.png"),OUT/"moving-a-crop.png")
                time.sleep(1.0)
                live_b=crop(shot(win,OUT/"moving-b.png"),OUT/"moving-b-crop.png")
                assert delta(live_a,live_b)>80,"restored walker path did not animate"

                invalid=original.replace("period_seconds = 6.0",
                                         "period_seconds = 0.01",1)
                assert invalid!=original
                manifest.write_text(invalid)
                wait(log,"Vesper hot reload rejected")
                assert proc.poll() is None,"invalid motion hot reload crashed"
                print("QOL07: live motion stop, restart and invalid edit rejection PASS",
                      flush=True)
            finally:
                if proc.poll() is None:
                    proc.terminate()
                    proc.wait(timeout=10)

if __name__=="__main__": main()
