#!/usr/bin/env python3
"""Production-only original media processing and audio synthesis.

No image generation or Python dependency is required at game runtime.
The large original AI-assisted illustrations are stored in assets/source.
The rendered image assets have been quality/alpha checked separately.
"""
from pathlib import Path
import wave
import math
import random
from PIL import Image, ImageEnhance, ImageDraw, ImageFilter, ImageOps
import numpy as np

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT/'assets/source'
BG = ROOT/'assets/backgrounds'
CH = ROOT/'assets/characters'
PAN = ROOT/'assets/panels'
AMB = ROOT/'assets/ambient'
AUD = ROOT/'assets/audio'
for p in (BG, CH, PAN, AMB, AUD): p.mkdir(parents=True, exist_ok=True)

base=Image.open(SRC/'observatory.png').convert('RGB')
W,H=1280,720
rng=random.Random(24680)

def crop_focus(im, focus_x=.50, focus_y=.50, zoom=1.0):
    ww=int(im.width/zoom); hh=int(ww*H/W)
    if hh>im.height:
        hh=int(im.height/zoom); ww=int(hh*W/H)
    x=max(0,min(im.width-ww,int(im.width*focus_x-ww/2)))
    y=max(0,min(im.height-hh,int(im.height*focus_y-hh/2)))
    return im.crop((x,y,x+ww,y+hh)).resize((W,H),Image.Resampling.LANCZOS)

def grade(im, tint=(0,0,0), opacity=0, contrast=1., saturation=1., brightness=1.):
    im=ImageEnhance.Contrast(im).enhance(contrast)
    im=ImageEnhance.Color(im).enhance(saturation)
    im=ImageEnhance.Brightness(im).enhance(brightness)
    if opacity: im=Image.blend(im, Image.new('RGB',im.size,tint),opacity)
    return im

def points(im, amount=100, palette=((255,231,178),(172,200,255)), seed=101):
    rnd=random.Random(seed)
    overlay=Image.new('RGBA',im.size)
    d=ImageDraw.Draw(overlay,'RGBA')
    for i in range(amount):
        x=rnd.randint(0,W-1);y=rnd.randint(0,H-1)
        radius=rnd.choice([1,1,1,2,3])
        c=rnd.choice(palette)
        d.ellipse((x-radius,y-radius,x+radius,y+radius),fill=c+(rnd.randint(70,160),))
    return Image.alpha_composite(im.convert('RGBA'),overlay).convert('RGB')

def rain(im, seed=91, amount=500):
    rnd=random.Random(seed)
    overlay=Image.new('RGBA',(W,H));d=ImageDraw.Draw(overlay,'RGBA')
    for _ in range(amount):
        x=rnd.randint(-50,W+50);y=rnd.randrange(H)
        d.line((x,y,x-9,y+32),fill=(194,209,251,rnd.randint(15,52)),width=1)
    return Image.alpha_composite(im.convert('RGBA'),overlay).convert('RGB')

def cracks(im):
    ov=Image.new('RGBA',(W,H));d=ImageDraw.Draw(ov,'RGBA');rnd=random.Random(122)
    for i in range(23):
        sx=W//2+rnd.randint(-140,140);sy=H//2+rnd.randint(-160,180)
        xx,yy=sx,sy;line=[(xx,yy)]
        for k in range(9):
            xx+=rnd.randint(-70,70); yy+=rnd.randint(-35,35)
            line.append((xx,yy))
        d.line(line,fill=(255,214,131,109),width=rnd.randint(1,3),joint='curve')
    return Image.alpha_composite(im.convert('RGBA'),ov).convert('RGB')

scenes={
 'observatory_evening':crop_focus(base),
 'observatory_close':grade(crop_focus(base,.24,.46,1.36),brightness=.93,saturation=1.1),
 'observatory_night':points(grade(crop_focus(base), (4,12,52),.54,brightness=.78),140,seed=9),
 'observatory_dawn':grade(crop_focus(base),(245,209,151),.14,brightness=1.2,saturation=1.08),
 'observatory_storm':rain(grade(crop_focus(base),(21,25,75),.43,contrast=1.18,brightness=.78)),
 'lens_corridor':points(grade(crop_focus(base,.27,.43,1.35),(15,21,58),.29),70,seed=3),
 'lost_city':grade(crop_focus(base,.81,.47,1.48),(91,90,147),.23,brightness=.89),
 'cloud_coast':grade(crop_focus(base,.83,.47,1.3),(110,151,206),.18,saturation=1.18),
 'archive_fire':points(grade(crop_focus(base,.27,.42,1.4),(170,47,22),.36,contrast=1.2),100,seed=45),
 'name_garden':points(grade(crop_focus(base,.77,.73,1.4),(19,90,79),.39,brightness=.96),260,seed=201),
 'undersea_lens':points(grade(crop_focus(base,.27,.49,1.25),(5,37,83),.56,brightness=.65),180,seed=72),
 'hidden_market':grade(crop_focus(base,.80,.44,1.38),(200,130,63),.19,brightness=1.02),
 'shattered_lens':cracks(points(grade(crop_focus(base),(35,25,65),.47,brightness=.72),240,seed=22)),
 'first_morning':points(grade(crop_focus(base,.73,.42,1.05),(255,211,112),.20,brightness=1.19),120,seed=47),
}
for key,im in scenes.items():
    im.save(BG/f'{key}.png',compress_level=9)

# Original source illustration silhouettes are preserved. Lighting grades are
# deliberately subtle so portrait compositions do not jump during crossfades.
for name,modes in [('mira',{'open':((255,255,255),0,.99), 'troubled':((25,35,70),.16,.89), 'luminous':((253,214,126),.20,1.08)}),('cael',{'steady':((255,255,255),0,1.), 'wary':((30,50,100),.13,.89), 'resolved':((249,186,107),.18,1.08)})]:
    art=Image.open(SRC/f'{name}.png').convert('RGBA')
    # Transparent source pixels are excluded from scaling bounds.
    bounds=art.getchannel('A').getbbox()
    if bounds: art=art.crop(bounds)
    art.thumbnail((500,700),Image.Resampling.LANCZOS)
    for mode,(tint,alpha,luma) in modes.items():
        rgb=ImageEnhance.Brightness(art.convert('RGB')).enhance(luma)
        if alpha: rgb=Image.blend(rgb,Image.new('RGB',rgb.size,tint),alpha)
        output=Image.merge('RGBA',(*rgb.split(),art.getchannel('A')))
        output.save(CH/f'{name}_{mode}.png',compress_level=9)

p=Image.open(SRC/'dialogue.png').convert('RGBA')
p.thumbnail((1920,820),Image.Resampling.LANCZOS)
p.save(PAN/'celestial_dialogue.png',compress_level=9)

# An independent moving sprite, authored in the manifest, not in Vesper code.
size=96
im=Image.new('RGBA',(size,size));d=ImageDraw.Draw(im,'RGBA')
for r in range(38,4,-1):
    a=int(120*(1-r/39)**1.7)
    d.ellipse((48-r,48-r,48+r,48+r),fill=(99,190,255,a))
d.ellipse((39,39,57,57),fill=(205,237,255,245))
im.save(AMB/'spark.png')

SR=16000
S=SR

def note(t,f,d=1.0,brightness=.42):
    # Layered, soft original bell/woodwind timbre. No sample or copyrighted tune.
    x=np.sin(2*np.pi*f*t)*0.56+np.sin(2*np.pi*2*f*t)*0.23+np.sin(2*np.pi*3.01*f*t)*0.06
    return x*brightness*d

def create_score(name,notes,tonic,duration=22):
    n=int(duration*SR);buffer=np.zeros(n,dtype=np.float64)
    step=1.0
    # Five original compositions share motifs but differ in orchestration and rhythm.
    speed={'archive':1.15,'threshold':.9,'mystery':1.36,'memory':1.5,'journey':.76,'storm':.57,'dawn':1.42}[name]
    beats=int(duration/speed)+1
    chords=[0,3,5,2,0,4,5,1]
    for i in range(beats):
        st=int(i*speed*SR)
        if st>=n:break
        count=min(int(SR*speed*1.2),n-st)
        t=np.arange(count)/SR
        env=np.clip(t/.05,0,1)*np.exp(-t/max(1.9,speed*2.1))
        freq=tonic*2**(notes[(i*5+i//3)%len(notes)]/12)
        amp=.40 if i%4==0 else .25
        buffer[st:st+count]+=note(t,freq,env,brightness=amp)
        if i%4==0:
            f2=tonic/2*2**(chords[(i//4)%len(chords)]/12)
            l=min(int(SR*speed*3.6),n-st)
            tt=np.arange(l)/SR
            attack=np.clip(tt/.25,0,1)
            release=np.clip((l/SR-tt)/.6,0,1)
            buffer[st:st+l]+=note(tt,f2,np.minimum(attack,release),brightness=.32)
    if name=='storm':
        rnd=np.random.default_rng(22)
        noise=rnd.standard_normal(n)
        kernel=np.ones(180)/180
        wash=np.convolve(noise,kernel,'same')
        buffer+=wash*.75
    # Atmospheric echo without expensive convolution.
    for lag,gain in [(int(.19*SR),.16),(int(.43*SR),.10)]:buffer[lag:]+=buffer[:-lag]*gain
    buffer*=np.linspace(.25,1,int(SR*2)).tolist()+[1]*(max(0,n-2*SR)) if False else 1
    ramp=min(int(.5*SR),n//5)
    buffer[:ramp]*=np.linspace(0,1,ramp)
    buffer[-ramp:]*=np.linspace(1,0,ramp)
    buffer=np.tanh(buffer*.75)
    to_wav(AUD/f'{name}.wav',buffer)

def to_wav(path,raw):
    arr=np.asarray(raw)
    arr=np.int16(np.clip(arr,-1,1)*32760)
    with wave.open(str(path),'wb') as fp:
        fp.setnchannels(1);fp.setsampwidth(2);fp.setframerate(SR)
        fp.writeframes(arr.tobytes())

scores={
 'archive':([0,7,4,11,9,4,2,7],196),
 'threshold':([0,3,7,10,5,2,8,12],174.61),
 'mystery':([0,1,7,5,2,8,11,6],155.56),
 'memory':([0,5,9,7,2,4,12,7],146.83),
 'journey':([0,4,7,12,9,7,4,2],220),
 'storm':([0,1,6,7,3,9,10,5],130.81),
 'dawn':([0,4,7,12,11,9,7,5],196),
}
for name,(notes,key) in scores.items():create_score(name,notes,key)
# Short page-turn and brass chime from synthesis/noise (original sound effects).
rng2=np.random.default_rng(48)
t=np.arange(int(.55*SR))/SR
noise=rng2.standard_normal(len(t))
page=np.convolve(noise,np.ones(95)/95,'same')*np.exp(-t*9)*1.6
to_wav(AUD/'page.wav',page)
t=np.arange(int(2.0*SR))/SR
bell=(np.sin(2*np.pi*523.25*t)+.53*np.sin(2*np.pi*784.88*t)+.19*np.sin(2*np.pi*1072.7*t))*np.exp(-2.4*t)*.28
to_wav(AUD/'bell.wav',bell)
print('Generated',len(scenes),'high-resolution painted scene treatments, 6 character grades, panel, spark, 7 original audio cues and 2 sound effects')
