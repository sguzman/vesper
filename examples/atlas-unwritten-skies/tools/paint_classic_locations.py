#!/usr/bin/env python3
"""Atlas original full-frame environment illustrations, standalone authored assets.

Eight intentionally different scenes. Deterministic offline rendering; no
changes or dependencies in the player. Uses Pillow/NumPy at authoring time.
"""
from __future__ import annotations
from pathlib import Path
import math, random, numpy as np
from PIL import Image, ImageDraw, ImageFilter

W,H=1280,720
OUT=Path(__file__).resolve().parents[1] / 'assets/backgrounds'
BRASS=(222,176,105); GOLD=(252,211,144)

def rr(seed): return random.Random(seed)
def rgba(rgb,a=255): return tuple(rgb)+(a,)
def background(top,middle,bottom,seed=1):
    yy,xx=np.mgrid[0:H,0:W]; r=np.random.default_rng(seed)
    t=np.clip(yy/(H*.68),0,1)[...,None];t=t*t*(3-2*t)
    t2=np.clip((yy-H*.68)/(H*.32),0,1)[...,None]
    a=np.asarray(top,float)[None,None,:]*(1-t)+np.asarray(middle,float)[None,None,:]*t
    a=a*(1-t2)+np.asarray(bottom,float)[None,None,:]*t2
    for scale,alpha in [(270,7),(85,4),(22,2)]:
        h=max(2,H//scale); w=max(2,W//scale)
        small=np.uint8(np.clip(127+r.normal(0,50,(h,w)),0,255))
        wave=np.asarray(Image.fromarray(small).resize((W,H),Image.Resampling.BICUBIC),float)-127
        a+=wave[...,None]*alpha/40
    vign=np.maximum(0,np.hypot((xx-W/2)/(W*.82),(yy-H*.4)/(H*.8))-.22)[...,None]
    a*=1-.18*vign
    a+=r.normal(0,1.5,(H,W,1))
    return Image.fromarray(np.uint8(np.clip(a,0,255)), 'RGB').convert('RGBA')

def layer(im,drawfn,blur=0):
    obj=Image.new('RGBA',(W,H));d=ImageDraw.Draw(obj,'RGBA');drawfn(d)
    if blur:obj=obj.filter(ImageFilter.GaussianBlur(blur))
    im.alpha_composite(obj)

def glow(im,x,y,r,color=(255,191,93),power=.7):
    def g(d):
        for f,a in [(3,6),(2.2,10),(1.5,20),(1,48),(.45,130)]:
            q=r*f;d.ellipse((x-q,y-q,x+q,y+q),fill=rgba(color,int(a*power)))
    layer(im,g,r*.12)
    d=ImageDraw.Draw(im,'RGBA');d.ellipse((x-2,y-2,x+2,y+2),fill=rgba(color,240))

def stars(im,seed,n=180,bottom=350):
    rand=rr(seed);d=ImageDraw.Draw(im,'RGBA')
    for _ in range(n):
        x=rand.randrange(W);y=rand.randrange(bottom); size=rand.choices([.6,1,1.7,2.6],[5,7,2,1])[0]
        d.ellipse((x-size,y-size,x+size,y+size),fill=(247,228,195,rand.randint(70,205)))
    for _ in range(6):
        x=rand.randrange(50,W-50);y=rand.randrange(40,bottom)
        d.line((x-10,y,x+10,y),fill=(243,212,154,155),width=1)
        d.line((x,y-10,x,y+10),fill=(243,212,154,155),width=1)

def moon(im,x,y,r,col=(246,213,157)):
    d=ImageDraw.Draw(im,'RGBA');d.ellipse((x-r*1.4,y-r*1.4,x+r*1.4,y+r*1.4),fill=(*col,20))
    glow(im,x,y,r*1.4,col,.65)
    d.ellipse((x-r,y-r,x+r,y+r),fill=(*col,250),outline=(249,236,199,235),width=2)
    for i,(xx,yy,sz) in enumerate([(x-r*.2,y+r*.21,r*.15),(x+r*.33,y-r*.20,r*.11),(x-r*.48,y-r*.38,r*.12)]):
        d.ellipse((xx-sz,yy-sz,xx+sz,yy+sz),fill=(174,154,143,55))

def town(im,seed,y=480,scale=1.0,near=False,waterline=None):
    q=rr(seed);d=ImageDraw.Draw(im,'RGBA');x=-40
    while x<W+50:
        wid=q.randint(42,90)*scale; h=q.randint(80,210)*scale
        top=y-h;col=q.choice([(14,23,51),(20,30,62),(24,39,74),(12,24,48)]) if not near else q.choice([(16,25,46),(25,30,49),(32,29,49)])
        d.polygon([(x,y),(x,top+12*scale),(x+wid*.5,top-q.randint(2,25)*scale),(x+wid,top+12*scale),(x+wid,y)],fill=rgba(col,245),outline=(66,68,88,70))
        d.rectangle((x+wid*.38,top-13*scale,x+wid*.6,top+10*scale),fill=rgba(col,240))
        for wy in range(int(top+25*scale),int(y-10*scale),int(25*scale)):
            for wx in range(int(x+10*scale),int(x+wid-7*scale),int(15*scale)):
                if q.random()<.7:
                    size=3 if scale<.8 else 5
                    d.rounded_rectangle((wx,wy,wx+size,wy+size*1.75),radius=1,fill=(253,182+q.randint(-25,25),95,q.randint(140,240)),outline=(128,91,77,130))
                    if waterline is not None:
                        mid=int(waterline+(waterline-wy)*.2)
                        if mid<H:d.line((wx,mid,wx+size,mid),fill=(250,168,98,38),width=2)
        x+=wid-3*scale

def lantern(im,x,y,r=15,col=(245,165,71)):
    d=ImageDraw.Draw(im,'RGBA');d.line((x,y-65,x,y-r),fill=(161,115,73,245),width=3)
    d.polygon([(x-r,y-r*.8),(x-r*.7,y-r*1.2),(x+r*.7,y-r*1.2),(x+r,y-r*.8)],fill=(120,79,60,255),outline=(237,191,119,220))
    d.rounded_rectangle((x-r*.8,y-r*.85,x+r*.8,y+r*.9),radius=4,fill=(*col,190),outline=(247,210,145,240),width=3)
    d.line((x,y-r*.7,x,y+r*.75),fill=(253,230,185,190),width=2)
    glow(im,x,y,r*1.4,col,.7)

def particles(im,seed,n=480,lo=0,hi=H):
    q=rr(seed);d=ImageDraw.Draw(im,'RGBA')
    for _ in range(n):
        x=q.randrange(W);y=q.randrange(lo,hi);rad=q.choices([.6,1,1.6,2],[3,5,2,1])[0]
        col=q.choice([(255,207,147),(178,203,220),(238,213,185)])
        d.ellipse((x-rad,y-rad,x+rad,y+rad),fill=rgba(col,q.randint(35,140)))

def window(d,x,y,r=10,col=(239,174,101)):
    d.rounded_rectangle((x-r*.55,y-r,x+r*.55,y+r),radius=max(2,int(r*.45)),fill=rgba(col,220),outline=(68,60,64,255),width=2)
    d.line((x,y-r,x,y+r),fill=(70,55,63,170),width=1)

def city():
    im=background((11,17,44),(96,70,115),(27,52,86),100);stars(im,41,260,370);moon(im,1000,130,65)
    town(im,55,y=500,scale=.75);d=ImageDraw.Draw(im,'RGBA')
    # Bellwether's central water, the ghost-tram bridge and the city being erased
    d.polygon([(340,500),(900,500),(1280,720),(0,720)],fill=(17,53,94,245))
    for y in range(525,720,9):
        q=rr(y)
        for _ in range(20):
            x=q.randrange(W);d.line((x,y,x+q.randint(8,65),y),fill=(162,156,157,q.randint(40,135)),width=q.choice([1,2]))
    town(im,64,y=570,scale=1.25,near=True)
    d.polygon([(0,544),(215,516),(312,534),(635,720),(0,720)],fill=(62,51,72,240),outline=(188,147,108,210),width=4)
    d.polygon([(1280,544),(1090,524),(958,534),(735,720),(1280,720)],fill=(61,53,78,240),outline=(188,147,108,210),width=4)
    # Stone bridge with many archways
    d.polygon([(140,489),(1120,474),(1220,500),(95,521)],fill=(118,105,109,230),outline=(213,161,103,220),width=5)
    for x in range(200,1110,112):
        d.arc((x,484,x+90,565),180,360,fill=(24,28,60,255),width=9)
        d.line((x,487,x,523),fill=(135,125,122,220),width=9)
    for x in range(225,1150,75):
        d.line((x,480,x,469),fill=(226,172,101,220),width=3)
        if x%2:glow(im,x,469,7)
    # Ghost map on upper right, integrating story's disintegration motif
    for i in range(15):
        x=520+i*21;y=230+round(20*math.sin(i*.7));d.line((x,y,x+90,y-45),fill=(240,205,156,30+i*2),width=1)
    particles(im,72,380,230,680);return im

def coast():
    im=background((35,72,132),(119,171,187),(14,91,123),101);d=ImageDraw.Draw(im,'RGBA')
    # Three atmospheric promontories
    for depth,col in [(.24,(91,133,145)),(.52,(56,105,111)),(.83,(38,79,93))]:
        y=int(380+depth*100)
        pts=[(0,H),(0,y),(175,y-95*depth),(225,y-74*depth),(330,y+35*depth),(480,y-16*depth),(610,y+80*depth),(W,H)]
        d.polygon(pts,fill=rgba(col,int(190+depth*60)))
    d.polygon([(0,460),(W,475),(W,H),(0,H)],fill=(28,103,138,220))
    for t in range(150):
        q=rr(t*17+23);x=q.randrange(W);y=q.randrange(470,H);length=q.randint(12,100)
        d.line((x,y,x+length,y),fill=(146,214,207,q.randint(30,145)),width=q.choice([1,2,4]))
    # Huge layered cliff right with winding stairs and tiny seaside shrine
    d.polygon([(755,150),(890,129),(1000,195),(1085,173),(1280,215),(1280,H),(1020,624),(945,470),(848,426)],fill=(59,76,91,255),outline=(156,138,132,175),width=5)
    d.polygon([(760,161),(880,124),(1003,190),(1087,162),(1280,206),(1280,310),(1058,244),(957,261)],fill=(105,145,127,255))
    for q in range(28):
        rand=rr(q+100);x=rand.randrange(850,1280);y=rand.randrange(250,630)
        d.ellipse((x-17,y-11,x+17,y+8),fill=(47,97+rand.randrange(30),95,200))
    # Cliffside architecture with luminous sanctuary and winding aqueduct
    d.rectangle((1035,105,1170,262),fill=(185,175,157,235),outline=(119,116,109,230),width=3)
    d.polygon([(1008,125),(1100,55),(1196,124)],fill=(54,80,106,245),outline=(234,198,127,235))
    d.rectangle((1090,28,1110,92),fill=(66,80,95,245))
    for x in [1055,1096,1137]:
        for y in [155,197,238]:window(d,x,y,9,(239,213,149))
    for t in np.linspace(0,1,50):
        x=760+430*t;y=442+65*math.sin(t*3.9)
        d.ellipse((x-2,y-2,x+2,y+2),fill=(214,190,154,180))
    # The horizon sails and birds
    for x,y,sz in [(125,390,35),(326,417,22),(497,370,14)]:
        d.polygon([(x,y),(x+sz,y),(x+sz*.25,y-sz*1.9)],fill=(221,227,222,220));d.line((x+sz*.25,y-sz*1.9,x+sz*.25,y+7),fill=(75,90,104,220),width=2)
    for x,y in [(350,160),(368,165),(674,118)]:d.arc((x-15,y,x+15,y+7),190,340,fill=(242,230,196,175),width=2)
    particles(im,101,320,40,590);return im

def market():
    im=background((19,25,56),(116,80,104),(24,51,78),102);stars(im,103,100,250);d=ImageDraw.Draw(im,'RGBA')
    town(im,104,505,.8)
    # Multi-level canals and market staircases
    d.polygon([(0,490),(W,476),(W,H),(0,H)],fill=(26,62,88,250))
    for y in range(520,H,14):
        q=rr(y+3)
        for _ in range(35):
            x=q.randrange(W);d.line((x,y,x+q.randint(4,37),y),fill=(227,158,93,q.randint(45,140)),width=2)
    for side in (0,1):
        x0=-40 if side==0 else 1030;x1=430 if side==0 else 1320
        d.polygon([(x0,0),(x1,0),(x1-48,260),(x0+40,420)],fill=(32,28,51,248))
        # shop windows and striped canopy
        for j in range(6):
            x=x0+45+j*65
            d.polygon([(x,246),(x+90,246),(x+75,285),(x+5,286)],fill=(161,83+j*7,78,255),outline=(218,171,110,200))
            d.rectangle((x+18,289,x+75,408),fill=(82,56,68,220),outline=(158,119,97,230))
            window(d,x+45,335,16)
    # Central floating awnings and elevated bridges
    for y,seed in [(142,120),(239,121),(336,122)]:
        q=rr(seed)
        d.arc((60,y-110,1220,y+70),0,180,fill=(197,139,110,240),width=6)
        for i in range(21):
            x=70+i*55;y2=int(y-24+22*math.sin(i*math.pi/20));col=q.choice([(189,95,80),(213,155,106),(79,104,123),(158,98,126)])
            d.polygon([(x,y2),(x+26,y2+7),(x+20,y2+42)],fill=rgba(col,245))
            if i%2==0:glow(im,x+15,y2+22,11,(247,178,103),.75)
    # Barges with traders and fabric
    d.polygon([(215,592),(575,587),(610,627),(255,645)],fill=(106,71,64,255),outline=(218,155,104,235),width=4)
    for xx in [300,368,441,519]:
        d.polygon([(xx,565),(xx+22,530),(xx+40,566)],fill=(61,42,57,245));d.ellipse((xx+14,517,xx+30,538),fill=(86,69,70,255))
    d.polygon([(680,637),(1170,615),(1135,650),(725,680)],fill=(89,64,72,255),outline=(219,163,91,220),width=5)
    for x,y in [(125,449),(375,436),(645,362),(960,427),(1195,476)]:lantern(im,x,y,14)
    particles(im,103,460,80,700);return im

def garden():
    im=background((13,28,67),(83,71,139),(26,62,92),104);stars(im,105,160,320);moon(im,940,90,54);d=ImageDraw.Draw(im,'RGBA')
    # Ruined arches frame a garden of nameless flowers
    for side in (0,1):
        x=70 if side==0 else 1130
        d.rounded_rectangle((x-65,150,x+65,700),radius=30,fill=(76,89,118,250),outline=(200,164,121,255),width=12)
        d.rounded_rectangle((x-42,170,x+42,645),radius=22,fill=(34,52,89,255),outline=(163,153,139,170),width=4)
        d.line((x,185,x,635),fill=(123,138,143,130),width=3)
    d.arc((60,-90,1200,820),190,352,fill=(201,170,122,240),width=24)
    d.polygon([(0,520),(1280,485),(W,H),(0,H)],fill=(30,65,73,255))
    # Curving pale stone path, name memorials both sides
    d.polygon([(580,500),(715,500),(1155,H),(125,H)],fill=(121,122,117,230),outline=(220,195,144,240),width=5)
    for side in [-1,1]:
        q=rr(105+side)
        for i in range(12):
            depth=i/12;x=640+side*(120+520*depth);y=int(492+210*depth)
            w=12+15*depth;h=35+51*depth
            d.rounded_rectangle((x-w,y-h,x+w,y),radius=5,fill=(103,119,140,230),outline=(227,194,141,225),width=3)
            d.arc((x-w*.6,y-h*.6,x+w*.6,y-h*.2),190,345,fill=(241,201,148,170),width=2)
            glow(im,x,y-h*.64,7,(207,185,242))
    # Lush foreground lavender trees and flowering tendrils
    for side,seed in [(0,108),(1280,109)]:
        q=rr(seed)
        for _ in range(250):
            x=q.gauss(side,155);y=q.uniform(0,490)
            if 0<=x<W:
                rad=q.randint(6,21);col=q.choice([(59,93,81),(91,85,125),(103,79,151),(149,119,170)])
                d.ellipse((x-rad,y-rad*.6,x+rad,y+rad*.6),fill=rgba(col,q.randint(110,240)))
        for j in range(27):
            x=q.uniform(40,250) if side==0 else q.uniform(1050,1260);y=q.uniform(80,490)
            d.line((x,y,x+q.randint(-20,20),y+65),fill=(142,142,121,160),width=2)
            for k in range(8):
                d.ellipse((x-3,y+k*8,x+4,y+7+k*8),fill=(190,156,222,210))
    for x,y in [(425,580),(835,545),(214,645),(1080,630)]:lantern(im,x,y,9,(245,190,130))
    particles(im,106,650);return im

def archive():
    im=background((21,20,42),(63,53,65),(42,36,43),110);d=ImageDraw.Draw(im,'RGBA')
    # Library vault with 3-story bookcases, balconies and arcades
    for side in (0,1):
        x=0 if side==0 else 1030
        d.polygon([(x,20),(x+250,70),(x+250,650),(x,720)],fill=(49,40,51,255),outline=(188,126,70,230),width=6)
        for y in [70,273,476]:
            d.line((x,y,x+250,y+15),fill=(220,154,83,240),width=8)
            q=rr(y+side)
            for k in range(35):
                bx=x+13+k*7.0;h=q.randint(37,110);color=q.choice([(122,74,66),(77,95,118),(144,124,100),(73,66,105),(167,97,54)])
                d.rectangle((bx,y+20,bx+q.randint(4,8),y+20+h),fill=rgba(color,230))
        d.line((x+250,20,x+250,720),fill=(171,110,65,250),width=7)
    # Layered blue-tinted colonnade and wide central manuscript table
    for x in range(330,1060,156):
        d.rounded_rectangle((x-55,10,x+55,455),radius=40,fill=(74,72,91,235),outline=(196,150,96,220),width=8)
        d.rounded_rectangle((x-38,31,x+38,428),radius=35,fill=(47,62,93,255))
    d.ellipse((493,43,787,337),fill=(38,77,113,245),outline=(221,178,102,255),width=13)
    for i in range(12):
        a=2*math.pi*i/12
        d.line((640,190,640+143*math.cos(a),190+143*math.sin(a)),fill=(219,166,88,210),width=5)
    moon(im,640,190,39,(237,178,125))
    d.polygon([(150,495),(1120,475),(1280,H),(0,H)],fill=(62,44,54,255))
    d.polygon([(350,490),(900,470),(1100,590),(170,623)],fill=(131,81,65,255),outline=(219,163,111,255),width=7)
    for j in range(8):
        x=360+55*j;y=478+j*5
        d.polygon([(x,y),(x+65,y-6),(x+74,y+11),(x+11,y+19)],fill=(229,210,166,230),outline=(169,125,88,200))
        d.line((x+10,y+6,x+65,y+1),fill=(108,91,91,80),width=1)
    for x in [260,1020,500,810]:lantern(im,x,430 if x%2 else 457,12)
    particles(im,111,650);return im

def undersea():
    im=background((4,27,66),(12,97,119),(7,38,79),120);d=ImageDraw.Draw(im,'RGBA')
    # Concentric containment vault, massive blue-glass lenses and light beams
    for j in range(5):
        pad=60+j*64
        d.arc((pad,-120+j*25,W-pad,850-j*60),180,360,fill=(112+12*j,151+5*j,160,180),width=8)
    d.ellipse((430,60,850,480),fill=(14,75,122,230),outline=(211,163,96,255),width=21)
    d.ellipse((470,100,810,440),fill=(35,151,170,155),outline=(156,211,206,245),width=5)
    for j in range(28):
        a=2*math.pi*j/28
        x,y=640+194*math.cos(a),270+194*math.sin(a)
        d.ellipse((x-7,y-7,x+7,y+7),fill=(240,197,112,235))
        if j%3==0:d.line((x,y,640,270),fill=(190,215,213,100),width=2)
    d.ellipse((562,192,718,348),fill=(100,224,212,100),outline=(235,228,190,220),width=6)
    glow(im,640,275,70,(126,235,230),.8)
    # Deep underwater pillars and coral growths
    d.polygon([(0,480),(W,500),(W,H),(0,H)],fill=(10,31,60,250))
    q=rr(122)
    for side in (0,1):
        x=50 if side==0 else 1100
        for j in range(7):
            xx=x+j*30
            d.line((xx,720,xx+q.randint(-30,30),515-q.randint(0,140)),fill=(64,146,130,220),width=q.randint(5,12))
            for k in range(4):
                y=550+k*40;d.line((xx,y,xx+q.randint(-38,38),y-q.randint(8,32)),fill=(89,173,147,200),width=4)
        for _ in range(35):
            xx=x+q.randint(-20,150);yy=q.randint(540,715)
            d.ellipse((xx-9,yy-9,xx+9,yy+9),fill=(190,112,135,210))
    for _ in range(100):
        x=q.randrange(W);y=q.randrange(H);rad=q.choice([2,3,5,8]);d.ellipse((x-rad,y-rad,x+rad,y+rad),outline=(196,243,237,q.randint(70,170)),width=2)
    for _ in range(25):
        x=q.randrange(80,W-80);y=q.randrange(50,520);d.arc((x-20,y-7,x+20,y+7),10,160,fill=(180,228,229,120),width=2)
    particles(im,123,340,80,640);return im

def shattered():
    im=background((26,20,48),(136,53,71),(35,30,47),130);d=ImageDraw.Draw(im,'RGBA')
    # Eclipse with fiery corona
    glow(im,641,175,115,(251,107,81),1.2)
    d.ellipse((534,65,748,279),fill=(14,16,42,255),outline=(246,155,110,240),width=8)
    d.ellipse((554,85,728,259),fill=(11,11,31,255))
    town(im,132,560,1.1);d=ImageDraw.Draw(im,'RGBA')
    # Massive broken orrery in foreground, violently sheared
    for rad in [105,175,248,309]:
        d.arc((640-rad,360-rad*.55,640+rad,360+rad*.55),20,155,fill=(219,153,97,190),width=9)
        d.arc((640-rad,360-rad*.55,640+rad,360+rad*.55),198,332,fill=(214,165,105,215),width=9)
    d.ellipse((615,333,665,383),fill=(239,190,116,245),outline=(94,64,68,255),width=4)
    q=rr(133)
    for _ in range(55):
        x=q.randrange(W);y=q.randrange(135,705)
        sz=q.randint(8,55);col=q.choice([(167,195,190),(112,149,160),(226,161,124),(80,137,151)])
        d.polygon([(x,y),(x+sz*.75,y-q.randint(5,22)),(x+sz,y+sz*1.4),(x-sz*.3,y+sz*.9)],fill=rgba(col,q.randint(80,180)),outline=(246,198,144,130),width=2)
        d.line((x,y,x+sz,y+sz*1.4),fill=(255,244,190,80),width=1)
    d.polygon([(0,620),(480,563),(640,660),(860,587),(1280,638),(1280,H),(0,H)],fill=(23,24,48,255))
    for x in range(90,1280,125):lantern(im,x,602+20*math.sin(x),8,(239,145,97))
    particles(im,134,490,60,660);return im

def port():
    im=background((26,42,97),(245,128,94),(59,63,103),140);d=ImageDraw.Draw(im,'RGBA')
    moon(im,860,158,60,(255,200,137));stars(im,141,90,260)
    town(im,142,570,.8)
    # Clouds and distant floating citadel
    for j in range(6):
        q=rr(j+144);x=q.randrange(0,W);y=q.randint(270,430)
        layer(im,lambda dr,x=x,y=y:dr.ellipse((x-165,y-35,x+165,y+35),fill=(249,196,154,50)),blur=19)
    # Airship hulls, riggings, fabric, celestial emblems
    def ship(cx,cy,scale,seed):
        d=ImageDraw.Draw(im,'RGBA');ww=270*scale;hh=65*scale
        d.ellipse((cx-ww/2,cy-hh,cx+ww/2,cy+hh),fill=(96,84,99,240),outline=(243,183,111,240),width=max(2,int(5*scale)))
        d.polygon([(cx-ww*.55,cy),(cx-ww*.45,cy+hh*.8),(cx+ww*.40,cy+hh*.8),(cx+ww*.57,cy)],fill=(63,56,79,245))
        for offset in np.linspace(-.4,.4,9):
            x=cx+ww*offset;d.arc((x-ww*.1,cy-hh,x+ww*.1,cy+hh),90,270,fill=(195,154,111,180),width=2)
        d.polygon([(cx-ww*.22,cy+hh*.87),(cx+ww*.2,cy+hh*.87),(cx+ww*.12,cy+hh*1.3),(cx-ww*.1,cy+hh*1.3)],fill=(68,54,62,255),outline=(216,163,111,235))
        for ofs in (-.2,0,.2):d.line((cx+ww*ofs,cy+hh*.5,cx+ww*ofs*.65,cy+hh*1.1),fill=(222,186,142,170),width=2)
        lantern(im,int(cx),int(cy+hh*1.35),max(5,int(8*scale)))
    ship(810,295,1.1,150);ship(310,245,.7,151);ship(1130,375,.52,152)
    d.polygon([(0,540),(400,530),(640,H),(0,H)],fill=(60,46,59,255),outline=(211,157,102,245),width=6)
    d.polygon([(W,530),(900,531),(650,H),(W,H)],fill=(61,46,59,255),outline=(220,164,100,245),width=6)
    for side in [0,1]:
        x=145 if side==0 else 1135
        d.line((x,450,x,H),fill=(146,107,78,245),width=12)
        for y in [455,520,591]:
            d.line((x-70,y,x+70,y),fill=(215,166,102,220),width=5)
        lantern(im,x,435,18)
    particles(im,143,400,130,690);return im

ART={
 'lost_city':city,
 'cloud_coast':coast,
 'hidden_market':market,
 'name_garden':garden,
 'archive_fire':archive,
 'undersea_lens':undersea,
 'shattered_lens':shattered,
 'journey_port':port,
}

def main():
    OUT.mkdir(exist_ok=True,parents=True)
    for name,fn in ART.items():
        im=fn()
        assert im.size==(W,H)
        im=im.convert('RGB')
        # Preserve the numerous subtle color gradients instead of palette-quantizing.
        im.save(OUT/f'{name}.png',optimize=True)
        print(f'{name}: {(OUT/f"{name}.png").stat().st_size:,} bytes',flush=True)
if __name__=='__main__':main()
