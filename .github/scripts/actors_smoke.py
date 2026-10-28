"""VX06 graphical regression: real Bevy actor layering and expression frames.

Clones an authored game; never changes repository artwork, stories or saves.
Synthetic flat-color art is created solely in the temporary QA fixture so
z-order and crossfade pixels are independently measurable.
"""
from __future__ import annotations

import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
BIN = ROOT/"target/debug/vesper-player"
OUT = Path("/tmp/vesper-actors-qa")


def run(*args, check=True):
    return subprocess.run(args, check=check, capture_output=True, text=True, timeout=25)


def wait(log, needle, count=1, timeout=18.):
    deadline = time.monotonic()+timeout
    while time.monotonic()<deadline:
        if log.exists() and log.read_text(errors="replace").count(needle)>=count:
            return
        time.sleep(.1)
    raise AssertionError(f"{needle!r} not found: {log.read_text(errors='replace')[-2000:]}")


def window(title):
    deadline = time.monotonic()+20
    while time.monotonic()<deadline:
        result=run("xdotool","search","--name",title,check=False)
        if result.returncode==0 and result.stdout.strip():
            win=result.stdout.splitlines()[0]
            run("xdotool","windowfocus","--sync",win)
            return win
        time.sleep(.2)
    raise AssertionError(f"No window: {title}")


def key(name, pause=.15):
    run("xdotool","key","--clearmodifiers",name)
    time.sleep(pause)


def shot(win, filename):
    run("import","-window",win,str(filename))


def rgb(image, x=640, y=300):
    fmt=f"%[fx:p{{{x},{y}}}.r*255] %[fx:p{{{x},{y}}}.g*255] %[fx:p{{{x},{y}}}.b*255]"
    return tuple(float(x) for x in run("convert",str(image),"-format",fmt,"info:").stdout.split())


def color_is(image, expected, x=640, y=300):
    values=rgb(image,x,y)
    colors={"red":(237.,65.,76.),"green":(48.,213.,110.),"blue":(60.,126.,236.)}
    target=colors[expected]
    return all(abs(a-b)<35 for a,b in zip(values,target))


def wait_color(win, image, expected, timeout=6., x=640, y=300):
    deadline=time.monotonic()+timeout
    while True:
        shot(win,image)
        if color_is(image,expected,x,y):
            return
        if time.monotonic()>deadline:
            raise AssertionError(f"expected {expected}, got {rgb(image,x,y)} in {image}")
        time.sleep(.12)


def difference(a,b):
    result=run("compare","-metric","AE",str(a),str(b),"null:",check=False)
    import re
    match=re.match(r"(\d+)",result.stderr.strip())
    assert match,result.stderr
    return int(match.group(1))


def build_fixture(tmp):
    game=tmp/"actor-fixture"
    shutil.copytree(ROOT/"examples/clockwork-moon",game)
    sprites=game/"assets/characters"
    for filename,color in (
        ("tess-curious.png","#ed414c"),
        ("tess-determined.png","#30d56e"),
        ("tess-relieved.png","#30d56e"),
        ("owl-calm.png","#3c7eec"),
    ):
        run("convert","-size","320x500",f"xc:{color}",str(sprites/filename))
    manifest=game/"game.toml"
    manifest.write_text(manifest.read_text()+"""
[characters.tess.stage]
scale = 0.95
depth = 5.0
crossfade_seconds = 1.2
enter_seconds = 0.7
exit_seconds = 0.6
slide_pixels = 22.0
idle_pixels = 0.0

[characters.owl.stage]
scale = 0.95
depth = 1.0
enter_seconds = 0.7
exit_seconds = 0.7
slide_pixels = 16.0
idle_pixels = 0.0
""")
    story=game/"scripts/story.vn"
    story.write_text("""@scene threshold
background cottage
show tess curious at center
show owl calm at center
"Red is above blue."
show tess determined at center
"Green crossfades over red while blue stays behind."
hide tess
"Blue stands alone after Tess leaves."
show tess relieved at center
"Tess comes back underneath Owl's new depth."
hide owl
"The blue owl exits; green stays on the stage."
"The End."
""")
    return game,manifest


def launch(game,out,reduced=False,watch=False):
    out.mkdir(parents=True,exist_ok=True)
    prefs=out/"preferences"
    prefs.mkdir(exist_ok=True)
    if reduced:
        (prefs/"player-preferences.json").write_text(
            '{"format_version":1,"reduced_motion":true}'
        )
    env=dict(os.environ,LIBGL_ALWAYS_SOFTWARE="1",
             VESPER_PREFERENCES_DIR=str(prefs),
             VESPER_SAVE_DIR=str(out/"saves"),
             VESPER_TRACE_SCENES="1")
    log=out/"player.log"
    stream=log.open("w")
    proc=subprocess.Popen([str(BIN),str(game)]+(["--watch"] if watch else []),
                          env=env,stdout=stream,stderr=subprocess.STDOUT)
    return proc,stream,log


def check_staging(game,manifest):
    out=OUT/"full"
    proc,stream,log=launch(game,out,watch=True)
    try:
        win=window("The Clockwork Moon")
        wait(log,"Vesper scene dialogue: threshold")
        wait(log,"Vesper character entrance complete: tess")
        wait(log,"Vesper character entrance complete: owl")
        red=out/"top-red.png"
        wait_color(win,red,"red")
        print("VX06 depth: tess above owl, both occupying the same authored anchor",flush=True)

        key("Return",.08)
        wait(log,"Vesper character staged: tess (determined)")
        key("p",.1)
        wait(log,"Vesper story paused")
        # Update-thread logs precede the rendered overlay under llvmpipe.
        # Compare only two frames captured after the pause UI is visible.
        time.sleep(.55)
        paused=out/"paused.png"
        shot(win,paused)
        time.sleep(.45)
        frozen=out/"paused-frozen.png"
        shot(win,frozen)
        assert difference(paused,frozen)==0,"pause advanced a character transition"
        key("p",.1)
        middle=out/"crossfade-middle.png"
        time.sleep(.3)
        shot(win,middle)
        mid=rgb(middle)
        assert mid[0]>65 and mid[1]>70, f"no blended expression midpoint: {mid}"
        wait(log,"Vesper character entrance complete: tess",2)
        green=out/"top-green.png"
        wait_color(win,green,"green")
        assert difference(middle,green)>500, "crossfade has no visibly distinct frames"
        print("VX06 expression: distinct mid/end frames; pause froze animation",flush=True)

        key("F5",.3)
        wait(log,"Vesper save written")
        initial=manifest.read_text()
        manifest.write_text(initial.replace("depth = 5.0","depth = 0.5")
                           .replace("depth = 1.0","depth = 7.0"))
        wait(log,"Vesper hot reload applied")
        blue=out/"hot-reloaded-depth.png"
        # The hot-reload log is emitted by Update before that frame reaches
        # llvmpipe. Wait for the actual visible result before asserting z-order.
        wait_color(win,blue,"blue",timeout=8.)
        hot_reload_rgb=rgb(blue)

        # Diagnose the world/render discrepancy by exposing the owl without
        # the tess sprite, then restoring a fresh tess expression at lower z.
        key("Return",.1)
        wait(log,"Vesper character exit started: tess")
        wait(log,"Vesper character exit complete: tess")
        owl_alone=out/"owl-uncovered.png"
        wait_color(win,owl_alone,"blue")
        key("Return",.1)
        wait(log,"Vesper character staged: tess (relieved)")
        wait(log,"Vesper character entrance complete: tess",3)
        respawned=out/"reintroduced-tess-depth.png"
        shot(win,respawned)
        print(f"VX06 depth probes: hot-reload={hot_reload_rgb}, tess-removed={rgb(owl_alone)}, tess-reintroduced={rgb(respawned)}",flush=True)
        assert color_is(blue,"blue"), "hot-reloaded world depth failed to reorder existing sprites"
        assert color_is(respawned,"blue"), "new sprite did not respect reloaded depth"

        key("Return",.1)
        wait(log,"Vesper character exit started: owl")
        exiting=out/"exit-middle.png"
        time.sleep(.25)
        shot(win,exiting)
        wait(log,"Vesper character exit complete: owl")
        after_exit=out/"exit-complete.png"
        wait_color(win,after_exit,"green")
        assert difference(exiting,after_exit)>500,"exit fade did not change visible pixels"
        key("F9",.3)
        wait(log,"Vesper save restored")
        restored=out/"restored-depth.png"
        wait_color(win,restored,"blue")
        print("VX06 exit/save: exit revealed lower sprite; F9 reconstructed depth",flush=True)

        for width,height in ((946,511),(955,520),(1920,1080)):
            run("xdotool","windowsize",win,str(width),str(height))
            time.sleep(.3)
            frame=out/f"stage-{width}x{height}.png"
            shot(win,frame)
            size=run("identify","-format","%w %h",str(frame)).stdout
            assert size==f"{width} {height}",size
            # Both staged actors remain on screen and correctly layered.
            assert color_is(frame,"blue",width//2,int(height*.40)),(
                f"actor missing or wrong z-order after {width}x{height}"
            )
        assert proc.poll() is None,"player terminated"
        print("VX06 responsive: compact and wide windows preserve staged actors",flush=True)
    finally:
        if proc.poll() is None:
            proc.terminate()
            proc.wait(timeout=10)
        stream.close()


def check_reduced(game):
    out=OUT/"reduced"
    proc,stream,log=launch(game,out,reduced=True)
    try:
        win=window("The Clockwork Moon")
        wait(log,"Vesper scene dialogue: threshold")
        red=out/"reduced-initial.png"
        wait_color(win,red,"red")
        key("Return",.15)
        after=out/"reduced-expression.png"
        wait_color(win,after,"green")
        time.sleep(.4)
        still=out/"reduced-static.png"
        shot(win,still)
        assert difference(after,still)==0,"reduced motion retained animation"
        assert "Vesper character staged" not in log.read_text(), (
            "staged transition started with reduced motion enabled"
        )
        assert proc.poll() is None
        print("VX06 reduced motion: immediate expression and static placement",flush=True)
    finally:
        if proc.poll() is None:
            proc.terminate()
            proc.wait(timeout=10)
        stream.close()


def main():
    OUT.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="vesper-actors-") as root:
        game,manifest=build_fixture(Path(root))
        check_staging(game,manifest)
        reduced_root=Path(root)/"reduced-fixture"
        reduced_root.mkdir()
        reduced_game,_=build_fixture(reduced_root)
        check_reduced(reduced_game)


if __name__=="__main__":
    main()
