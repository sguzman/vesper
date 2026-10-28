"""VX08 real UI acceptance: interpreter-fed history, manual saves, quick saves.

Only a temporary copy of the external authored game is mutated. No CI test
depends on wall-clock timing of a network service or performs OCR.
"""
from __future__ import annotations
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time

ROOT=Path(__file__).resolve().parents[2]
BIN=ROOT/"target/debug/vesper-player"
OUT=Path("/tmp/vesper-history-saves-qa")
GAME_ID="clockwork-moon"


def run(*args,check=True):
    return subprocess.run(args,check=check,capture_output=True,text=True,timeout=30)


def key(value,delay=.15):
    run("xdotool","key","--clearmodifiers",value)
    time.sleep(delay)


def wait(log,needle,count=1,timeout=15.):
    deadline=time.monotonic()+timeout
    while time.monotonic()<deadline:
        if log.exists() and log.read_text(errors="replace").count(needle)>=count:
            return
        time.sleep(.12)
    raise AssertionError(f"missing {needle!r} ({count}): {log.read_text(errors='replace')[-2000:]}")


def window():
    deadline=time.monotonic()+20.
    while time.monotonic()<deadline:
        r=run("xdotool","search","--name","The Clockwork Moon",check=False)
        if r.returncode==0 and r.stdout.strip():
            win=r.stdout.splitlines()[0]
            run("xdotool","windowfocus","--sync",win)
            return win
        time.sleep(.16)
    raise AssertionError("player window missing")


def shot(win,path):
    run("import","-window",win,str(path))


def difference(a,b):
    result=run("compare","-metric","AE",str(a),str(b),"null:",check=False)
    m=re.match(r"\d+",result.stderr.strip())
    assert m,result.stderr
    return int(m.group())


def fixture(root):
    game=root/"game"
    shutil.copytree(ROOT/"examples/clockwork-moon",game)
    (game/"scripts/story.vn").write_text("""@scene threshold
background cottage
"First remembered line."
tess: "Second remembered line."
* "Take the forest path." -> forest
* "Take the river path." -> river
@scene forest
background forest
tess: "Forest branch dialogue."
@scene river
background river
owl: "River branch dialogue."
""")
    return game


def check_history_and_slots(game):
    out=OUT/"interactive"
    out.mkdir(parents=True,exist_ok=True)
    pref=out/"preferences"
    pref.mkdir(exist_ok=True)
    saves=out/"saves"
    env=dict(os.environ,LIBGL_ALWAYS_SOFTWARE="1",
             VESPER_SAVE_DIR=str(saves),
             VESPER_PREFERENCES_DIR=str(pref),
             VESPER_TRACE_SCENES="1")
    log=out/"player.log"
    with log.open("w") as stream:
        proc=subprocess.Popen([str(BIN),str(game)],env=env,
                              stdout=stream,stderr=subprocess.STDOUT)
        try:
            win=window()
            wait(log,"Vesper scene dialogue: threshold")
            time.sleep(.9)
            base=out/"dialogue.png"
            shot(win,base)
            key("h")
            wait(log,"Vesper reading history opened: 2 entries")
            history1=out/"history-initial.png"
            shot(win,history1)
            assert difference(base,history1)>5000,"reading history overlay not visible"
            key("Return")
            key("Page_Up")
            key("Page_Down")
            assert log.read_text().count("Vesper scene dialogue: threshold")==1, (
                "history input accidentally executed narrative"
            )
            key("Escape")
            wait(log,"Vesper reading history closed")
            key("Return",.3)
            wait(log,"Vesper scene dialogue: threshold",count=2)
            key("h")
            wait(log,"Vesper reading history opened: 3 entries")
            history2=out/"history-two-lines.png"
            shot(win,history2)
            assert difference(history1,history2)>100,"history did not add a line"
            key("h")
            wait(log,"Vesper reading history closed",count=2)
            key("Return",.3)
            wait(log,"Vesper choice UI mounted 2 independent rows")

            # Structured manual slot 1 coexists with untouched legacy quick path.
            key("F6")
            wait(log,"Vesper save slots opened")
            wait(log,"Vesper save slot metadata scanned")
            manager=out/"slot-empty.png"
            shot(win,manager)
            assert difference(manager,base)>5000,"manual save overlay not visible"
            key("Return")
            wait(log,"Vesper save written",count=1)
            wait(log,"Vesper save slot metadata scanned",count=2)
            slot=saves/f"{GAME_ID}.slot1.json"
            assert slot.is_file(),"manual slot 1 did not write"
            assert not (saves/f"{GAME_ID}.json").exists(), (
                "manual slot overwrote the existing F5/F9 quick file"
            )
            payload=json.loads(slot.read_text())
            assert payload["format_version"]==1
            assert payload["machine"]["pending"] is not None

            key("Return")
            wait(log,"Vesper overwrite confirmation requested: 1")
            confirmation=out/"overwrite-confirmation.png"
            shot(win,confirmation)
            key("Escape")
            wait(log,"Vesper manual overwrite cancelled")
            assert slot.read_bytes()==slot.read_bytes(),"cancelled save changed bytes"
            key("Return")
            wait(log,"Vesper overwrite confirmation requested: 1",count=2)
            key("Return")
            wait(log,"Vesper overwrite confirmed: 1")
            wait(log,"Vesper save written",count=2)
            key("Escape")
            wait(log,"Vesper save slots closed")

            key("F5")
            wait(log,"Vesper save written",count=3)
            quick=saves/f"{GAME_ID}.json"
            assert quick.is_file(),"F5 quick save stopped writing legacy file"
            key("Down")
            key("Return",.4)
            wait(log,"Vesper scene dialogue: river")
            key("h")
            wait(log,"Vesper reading history opened: 5 entries")
            branch=out/"history-river-branch.png"
            shot(win,branch)
            key("Escape")
            key("F9")
            wait(log,"Vesper save restored")
            key("h")
            wait(log,"Vesper reading history opened: 0 entries")
            restored=out/"history-after-rewind.png"
            shot(win,restored)
            assert difference(branch,restored)>100, (
                "load retained later-branch dialogue in history"
            )
            key("Escape")
            key("F6")
            wait(log,"Vesper save slots opened",count=2)
            wait(log,"Vesper save slot metadata scanned",count=3)
            key("Right")
            key("Return")
            wait(log,"Vesper manual slot load requested: 1")
            wait(log,"Vesper save restored",count=2)
            wait(log,"Vesper save slots closed",count=2)
            assert proc.poll() is None,"player exited during save manager"
            print("VX08: history view, no narrative progression, quick F5/F9, manual slot, overwrite cancellation, and load verified",flush=True)
        finally:
            if proc.poll() is None:
                proc.terminate()
                proc.wait(timeout=10)


def main():
    OUT.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="vesper-history-") as temp:
        game=fixture(Path(temp))
        check_history_and_slots(game)


if __name__=="__main__":
    main()
