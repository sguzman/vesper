"""VX04: real Bevy text reveal, skip-once, load and opt-in auto controls.

All media/story/theme fixtures are copied to /tmp. Source games are untouched.
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

ROOT = Path(__file__).resolve().parents[2]
PLAYER = ROOT / "target/debug/vesper-player"
OUT = Path("/tmp/vesper-text-pacing-qa")
TITLE = "The Little Lantern"

def run(*args, check=True):
    return subprocess.run(args, text=True, capture_output=True, check=check, timeout=20)

def wait_for(log, marker, timeout=17):
    end = time.monotonic()+timeout
    while time.monotonic()<end:
        data=log.read_text(errors="replace")
        if marker in data:
            return data
        time.sleep(.12)
    raise AssertionError(f"missing {marker}: {log.read_text(errors='replace')[-2500:]}")

def focus_window():
    for _ in range(100):
        found=run("xdotool","search","--name",TITLE,check=False)
        if found.returncode==0 and found.stdout.strip():
            win=found.stdout.splitlines()[0]
            run("xdotool","windowfocus","--sync",win)
            return win
        time.sleep(.2)
    raise AssertionError("Vesper window never appeared")

def press(key, delay=.35):
    run("xdotool","key","--clearmodifiers",key)
    time.sleep(delay)

def shot(win,path):
    run("import","-window",win,str(path))
    return path

def diff(a,b):
    res=run("compare","-metric","AE",str(a),str(b),"null:",check=False)
    match=re.match(r"^\d+",res.stderr.strip())
    assert match,res.stderr
    return int(match.group())

def fixture(parent, mode):
    game=parent / "little-lantern"
    shutil.copytree(ROOT/"examples/little-lantern",game)
    theme=game/"styles/lantern.toml"
    raw=theme.read_text()
    theme.write_text(raw.replace("fade_seconds = 0.30",
                                  'fade_seconds = 0.0\nreveal_speed = "'+mode+'"'))
    return game

def start(game,folder,prefs=None,extra=None,watch=False):
    folder.mkdir(parents=True,exist_ok=True)
    pref_dir=folder/"preferences"
    pref_dir.mkdir()
    if prefs is not None:
        (pref_dir/"player-preferences.json").write_text(json.dumps({"format_version":1,**prefs}))
    log=folder/"player.log"
    env=dict(os.environ,LIBGL_ALWAYS_SOFTWARE="1",VESPER_TRACE_SCENES="1",
             VESPER_PREFERENCES_DIR=str(pref_dir),VESPER_SAVE_DIR=str(folder/"saves"))
    if extra: env.update(extra)
    stream=log.open("w")
    command=[str(PLAYER),str(game)]
    if watch: command.append("--watch")
    proc=subprocess.Popen(command,stdout=stream,stderr=subprocess.STDOUT,env=env)
    try: win=focus_window()
    except BaseException:
        proc.terminate();proc.wait(timeout=10);stream.close();raise
    return proc,stream,win,log

def stop(proc,stream):
    if proc.poll() is None:proc.terminate();proc.wait(timeout=10)
    stream.close()

def verify_manual(game):
    folder=OUT/"manual-slow"
    proc,stream,win,log=start(game,folder)
    try:
        wait_for(log,"Vesper scene dialogue: first_light")
        time.sleep(.55)
        first=shot(win,folder/"reveal-early.png")
        time.sleep(1.2)
        second=shot(win,folder/"reveal-later.png")
        delta=diff(first,second)
        assert delta>200,f"slow authored reveal did not change text: {delta} pixels"
        before=log.read_text(errors="replace").count("Vesper scene dialogue: first_light")
        press("Return")
        wait_for(log,"Vesper reveal completed by reader")
        after=log.read_text(errors="replace").count("Vesper scene dialogue: first_light")
        assert before==after,"first Enter advanced narrative instead of completing line"
        full=shot(win,folder/"line-complete.png")
        assert diff(second,full)>50,"skip did not reveal remainder"
        press("p")
        wait_for(log,"Vesper story paused")
        press("p")
        press("Return")
        after=log.read_text(errors="replace").count("Vesper scene dialogue: first_light")
        assert after==before+1,"second Enter did not advance exactly one dialogue"
        press("F5")
        wait_for(log,"Vesper save written")
        press("F9")
        wait_for(log,"Vesper save restored")
        time.sleep(.4)
        loaded=shot(win,folder/"load-shows-complete.png")
        # Complete on restore, not a fresh partial reveal.
        time.sleep(.8)
        after_load=shot(win,folder/"load-still-complete.png")
        assert diff(loaded,after_load)<1500,"F9 replayed reveal rather than restoring completed text"
        print("VX04 manual: timed reveal, skip once, pause, F5/F9 complete passed",flush=True)
    finally:stop(proc,stream)

def verify_reload_and_resize(game):
    folder=OUT/"watch-and-resize"
    proc,stream,win,log=start(game,folder,watch=True)
    try:
        wait_for(log,"Vesper scene dialogue: first_light")
        time.sleep(1.3)
        run("xdotool","windowsize",win,"946","511")
        time.sleep(.45)
        before=shot(win,folder/"resized-reveal-partial.png")
        time.sleep(.6)
        later=shot(win,folder/"resized-reveal-continuing.png")
        assert diff(before,later)>100,"resize unexpectedly froze a partially revealing line"

        # An external authored theme change must keep the currently displayed
        # narrative state and reconstruct the visible line as complete.
        theme=game/"styles"/"lantern.toml"
        text=theme.read_text()
        assert "reveal_speed = \"slow\"" in text
        theme.write_text(text.replace("reveal_speed = \"slow\"",
                                           'reveal_speed = "fast"'))
        wait_for(log,"Vesper hot reload applied")
        time.sleep(.45)
        complete=shot(win,folder/"hot-reload-completed-line.png")
        assert diff(later,complete)>150,"hot reload did not complete current line"
        press("Return")
        data=log.read_text(errors="replace")
        assert data.count("Vesper scene dialogue: first_light")>=2, (
            "hot reload lost completion state; first Enter did not advance"
        )
        print("VX04 watch/resize: partial progression and completed hot reload passed",flush=True)
    finally:stop(proc,stream)


def verify_instant(game):
    folder=OUT/"reduced-motion"
    proc,stream,win,log=start(game,folder,{"reveal_speed":"slow","reduced_motion":True})
    try:
        wait_for(log,"Vesper scene dialogue: first_light")
        time.sleep(2.0)
        first=shot(win,folder/"reduced-cue-1.png")
        time.sleep(.8)
        second=shot(win,folder/"reduced-cue-2.png")
        assert diff(first,second)<200,"reduced motion cue/text should remain static"
        assert "Vesper reveal completed by reader" not in log.read_text(errors="replace")
        print("VX04 reduced-motion override: instant text and static cue passed",flush=True)
    finally:stop(proc,stream)

def verify_controls(game):
    folder=OUT/"interactive-reading-controls"
    proc,stream,win,log=start(game,folder)
    try:
        wait_for(log,"Vesper scene dialogue: first_light")
        press("m")
        press("4")
        wait_for(log,"Vesper appearance settings opened")
        # Wait for the real overlay to be presented before sending repeated
        # keys. Software Vulkan can render the first settings frame slowly
        # after six back-to-back real-window cinematic sessions.
        time.sleep(.9)
        shot(win,folder/"settings-open-before-input.png")
        for _ in range(3):
            press("Down", .32)
        press("Right", .50)
        prefs_file=folder/"preferences"/"player-preferences.json"
        wait_for(log,"Vesper player preferences saved")
        def load():
            try:return json.loads(prefs_file.read_text())
            except (OSError,json.JSONDecodeError):return {}
        end=time.monotonic()+7
        while time.monotonic()<end and load().get("reveal_speed")!="normal":
            time.sleep(.1)
        assert load().get("reveal_speed")=="normal",load()
        press("Down")
        press("Right")
        press("Down")
        press("Right")
        end=time.monotonic()+7
        while time.monotonic()<end:
            value=load()
            if value.get("reduced_motion") is True and value.get("auto_advance") is True:
                break
            time.sleep(.1)
        assert load().get("reduced_motion") is True and load().get("auto_advance") is True,load()
        run("xdotool","windowsize",win,"946","511")
        time.sleep(.5)
        shot(win,folder/"settings-reading-946x511.png")
        press("Down")  # High contrast (index 6)
        press("Down")  # Reset to game defaults (index 7)
        press("Return")
        end=time.monotonic()+7
        while time.monotonic()<end and any(
            key in load() for key in ("reveal_speed","reduced_motion","auto_advance")
        ):
            time.sleep(.1)
        assert not any(k in load() for k in ("reveal_speed","reduced_motion","auto_advance")),load()
        press("Down") # Back (index 8)
        press("Return")
        wait_for(log,"Vesper appearance settings closed")
        assert proc.poll() is None
        print("VX04 settings: speed, reduced motion, auto opt-in, reset, Back passed",flush=True)
    finally:stop(proc,stream)


def verify_auto(game):
    folder=OUT/"auto-opt-in"
    proc,stream,win,log=start(game,folder,{"reveal_speed":"fast","auto_advance":True})
    try:
        wait_for(log,"Vesper auto-advance ready",timeout=17)
        time.sleep(.6)
        count=log.read_text(errors="replace").count("Vesper scene dialogue: first_light")
        assert count>=2,f"auto-ready did not advance story: {count}"
        shot(win,folder/"auto-advanced.png")
        print("VX04 auto-advance only after opt-in: passed",flush=True)
    finally:stop(proc,stream)

if __name__=="__main__":
    OUT.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="vesper-text-reveal-") as tmp:
        game=fixture(Path(tmp),"slow")
        verify_manual(game)
        verify_instant(game)
        verify_auto(game)
        verify_controls(game)
        watch_game=Path(tmp)/"little-lantern-watch"
        shutil.copytree(game,watch_game)
        verify_reload_and_resize(watch_game)
