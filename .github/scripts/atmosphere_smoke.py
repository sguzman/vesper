"""VX07 graphical acceptance: shader-free atmosphere and micro-interactions.

All modifications are inside isolated copies of the externally authored
Clockwork Moon game. No repository artwork, saves or stories are mutated.
"""
from __future__ import annotations

import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time

ROOT=Path(__file__).resolve().parents[2]
BIN=ROOT/"target/debug/vesper-player"
OUT=Path("/tmp/vesper-atmosphere-qa")


def run(*args, check=True):
    return subprocess.run(args, text=True, capture_output=True, check=check, timeout=25)


def wait(log, needle, count=1, timeout=20.):
    deadline=time.monotonic()+timeout
    while time.monotonic()<deadline:
        if log.exists() and log.read_text(errors="replace").count(needle)>=count:
            return
        time.sleep(.12)
    raise AssertionError(f"missing {needle!r}: {log.read_text(errors='replace')[-2500:]}")


def window(title="The Clockwork Moon"):
    deadline=time.monotonic()+20.
    while time.monotonic()<deadline:
        result=run("xdotool","search","--name",title,check=False)
        if result.returncode==0 and result.stdout.strip():
            win=result.stdout.splitlines()[0]
            run("xdotool","windowfocus","--sync",win)
            return win
        time.sleep(.15)
    raise AssertionError("player window unavailable")


def key(name, delay=.1):
    run("xdotool","key","--clearmodifiers",name)
    time.sleep(delay)


def shot(win,target):
    run("import","-window",win,str(target))


def rgb(img,x=640,y=1):
    fmt=f"%[fx:p{{{x},{y}}}.r*255] %[fx:p{{{x},{y}}}.g*255] %[fx:p{{{x},{y}}}.b*255]"
    return tuple(float(v) for v in run("convert",str(img),"-format",fmt,"info:").stdout.split())


def colors(img):
    return int(run("convert",str(img),"-crop","1280x260+0+0",
                   "+repage","-format","%k","info:").stdout.strip())


def ready(win,img):
    deadline=time.monotonic()+7.
    while time.monotonic()<deadline:
        shot(win,img)
        if colors(img)>16: return
        time.sleep(.12)
    raise AssertionError("background never reached the renderer")


def changed(a,b):
    result=run("compare","-metric","AE",str(a),str(b),"null:",check=False)
    match=re.match(r"^\d+",result.stderr.strip())
    assert match,result.stderr
    return int(match.group())


def scene_pixels(a,b,out):
    crop_a=out/"comparison-a.png"
    crop_b=out/"comparison-b.png"
    for src,dst in ((a,crop_a),(b,crop_b)):
        run("convert",str(src),"-crop","1280x260+0+0","+repage",str(dst))
    return changed(crop_a,crop_b)


def fixture(root):
    game=root/"game"
    shutil.copytree(ROOT/"examples/clockwork-moon",game)
    story=game/"scripts/story.vn"
    story.write_text("""@scene threshold
background cottage
"Which light will you follow?"
* "Follow the shore." -> shore
* "Follow the stars." -> stars
@scene shore
"The shore is calm."
@scene stars
"The stars are close."
""")
    theme=game/"styles/moonlight.toml"
    return game,theme,theme.read_text()


def write_theme(path,original,preset,focus=.0,confirm=.0):
    path.write_text(original + f"""
[atmosphere]
preset = "{preset}"
seed = 481516
count = 16
speed = 20.0
radius = 6.0
opacity = 0.32
color = [0.97, 0.89, 0.63, 1.0]

[feedback]
focus_seconds = {focus}
confirm_seconds = {confirm}
cue_opacity = 0.65
cue_color = [1.0, 0.1, 0.1, 1.0]
""")


def launch(game,out,reduced=False,watch=False):
    out.mkdir(parents=True,exist_ok=True)
    prefs=out/"prefs"
    prefs.mkdir(exist_ok=True)
    if reduced:
        (prefs/"player-preferences.json").write_text(
            '{"format_version":1,"reduced_motion":true}'
        )
    env=dict(os.environ, LIBGL_ALWAYS_SOFTWARE="1",
             VESPER_SAVE_DIR=str(out/"saves"),
             VESPER_PREFERENCES_DIR=str(prefs),
             VESPER_TRACE_SCENES="1")
    log=out/"player.log"
    stream=log.open("w")
    proc=subprocess.Popen([str(BIN),str(game)]+(["--watch"] if watch else []),
                          env=env,stdout=stream,stderr=subprocess.STDOUT)
    return proc,stream,log


def check_presets(game,theme,original):
    out=OUT/"presets"
    write_theme(theme,original,"motes")
    proc,stream,log=launch(game,out,watch=True)
    try:
        win=window()
        wait(log,"Vesper atmosphere configured: Motes, 16 entities")
        start=out/"motes-start.png"
        ready(win,start)
        # Wait for the initial external scene's legacy fade to finish; measure
        # atmosphere motion rather than image loading/fade-in.
        time.sleep(1.0)
        shot(win,start)
        time.sleep(.6)
        moving=out/"motes-moving.png"
        shot(win,moving)
        assert scene_pixels(start,moving,out)>150, "motes did not drift"
        key("p",.12)
        wait(log,"Vesper story paused")
        time.sleep(.55)
        freeze_a=out/"motes-paused.png"
        shot(win,freeze_a)
        time.sleep(.45)
        freeze_b=out/"motes-frozen.png"
        shot(win,freeze_b)
        assert changed(freeze_a,freeze_b)==0, "motes moved during pause"
        key("p",.15)
        print("VX07 motes: visible drift and pixel-identical paused frames",flush=True)

        previous=moving
        for name,count in (("glow",3),("vignette",12),("moonlit",31),("off",0)):
            write_theme(theme,original,name)
            wait(log,"Vesper hot reload applied",count=1+["glow","vignette","moonlit","off"].index(name))
            wait(log,f"Vesper atmosphere configured: {name.title()}, {count} entities")
            time.sleep(.35)
            img=out/f"{name}.png"
            shot(win,img)
            assert scene_pixels(previous,img,out)>150, (
                f"{name} rendered identically to previous atmosphere preset"
            )
            previous=img
        assert proc.poll() is None,"player crashed during atmosphere reloads"
        print("VX07 authored presets: glow/vignette/moonlit/off hot reload and bounds pass",flush=True)
    finally:
        if proc.poll() is None:
            proc.terminate()
            proc.wait(timeout=10)
        stream.close()


def check_reduced(game,theme,original):
    out=OUT/"reduced"
    write_theme(theme,original,"motes")
    proc,stream,log=launch(game,out,reduced=True)
    try:
        win=window()
        wait(log,"Vesper atmosphere configured: Motes")
        image=out/"reduced-start.png"
        ready(win,image)
        time.sleep(1.0)
        shot(win,image)
        time.sleep(.8)
        still=out/"reduced-still.png"
        shot(win,still)
        assert changed(image,still)==0, "motes ignored reduced motion"
        print("VX07 reduced motion: authored motes disabled, background static",flush=True)
    finally:
        if proc.poll() is None:
            proc.terminate()
            proc.wait(timeout=10)
        stream.close()


def catch_cue(win,baseline,out,name):
    img=out/f"{name}-cue.png"
    deadline=time.monotonic()+.65
    while time.monotonic()<deadline:
        shot(win,img)
        if rgb(img)[0]>baseline[0]+40:
            return img
        time.sleep(.04)
    raise AssertionError(f"{name} confirmation cue was not visible: baseline={baseline}, last={rgb(img)}")


def check_feedback(game,theme,original):
    out=OUT/"feedback"
    write_theme(theme,original,"off",focus=.30,confirm=.35)
    proc,stream,log=launch(game,out)
    try:
        win=window()
        wait(log,"Vesper atmosphere configured: Off")
        first=out/"before-choices.png"
        ready(win,first)
        time.sleep(.8)
        key("Return",.12)
        wait(log,"Vesper choice UI mounted 2 independent rows")
        time.sleep(.4)
        selected=out/"choice-first-focus.png"
        shot(win,selected)
        key("Down",.03)
        early=out/"choice-focus-early.png"
        shot(win,early)
        time.sleep(.4)
        late=out/"choice-focus-end.png"
        shot(win,late)
        assert changed(selected,late)>250, "choice focus never moved"
        assert changed(early,late)>80, "choice focus transition has no intermediate frame"

        baseline=rgb(late)
        key("Return",.01)
        visible=catch_cue(win,baseline,out,"choice")
        time.sleep(.5)
        gone=out/"choice-cue-ended.png"
        shot(win,gone)
        assert rgb(gone)[0]<rgb(visible)[0]-30,"choice confirmation cue did not end"
        print("VX07 choices: themed focus easing and confirmation cue pass",flush=True)

        key("m",.12)
        wait(log,"Vesper main menu opened")
        time.sleep(.35)
        menu=out/"menu-first.png"
        shot(win,menu)
        key("Down",.03)
        early=out/"menu-focus-early.png"
        shot(win,early)
        time.sleep(.4)
        late=out/"menu-focus-end.png"
        shot(win,late)
        assert changed(menu,late)>250,"menu focus never moved"
        assert changed(early,late)>80,"menu focus has no intermediate frame"
        baseline=rgb(late)
        key("Return",.01)
        catch_cue(win,baseline,out,"menu")
        assert proc.poll() is None
        print("VX07 menus: same focus and confirmation behavior with keyboard",flush=True)
    finally:
        if proc.poll() is None:
            proc.terminate()
            proc.wait(timeout=10)
        stream.close()


def main():
    OUT.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="vesper-atmosphere-") as tmp:
        for name,checker in (
            ("presets",check_presets),
            ("reduced",check_reduced),
            ("feedback",check_feedback),
        ):
            root=Path(tmp)/name
            root.mkdir()
            game,theme,original=fixture(root)
            checker(game,theme,original)


if __name__=="__main__":
    main()
