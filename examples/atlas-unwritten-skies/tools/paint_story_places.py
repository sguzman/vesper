#!/usr/bin/env python3
"""Deterministic, original illustrated backdrop studies for five Atlas chapters.

This offline authoring tool generates only external 1280x720 artwork. No engine
changes or runtime Python dependency. Drawing is deliberately scene-specific:
architecture, environmental story objects, and lighting follow each chapter.
"""
from __future__ import annotations
from pathlib import Path
import math
import random
import numpy as np
from PIL import Image, ImageDraw, ImageFilter

OUT = Path(__file__).resolve().parents[1] / 'assets/backgrounds'
W, H = 1280, 720


def gradient(top, horizon, bottom, seed=0, haze=0.10):
    yy, xx = np.mgrid[0:H, 0:W]
    t=np.clip(yy/(H*.62),0,1)[...,None]
    b=np.clip((yy-H*.62)/(H*.38),0,1)[...,None]
    a=np.array(top); c=np.array(horizon); d=np.array(bottom)
    arr=a[None,None,:]*(1-t)+c[None,None,:]*t
    arr=arr*(1-b)+d[None,None,:]*b
    vig=np.clip(np.sqrt(((xx-W/2)/(W*.8))**2+((yy-H/2)/(H*.75))**2),0,.8)[...,None]
    arr*=(1-.18*vig)
    rng=np.random.default_rng(seed)
    arr+=rng.normal(0,2.5,(H,W,1))
    arr=np.clip(arr,0,255).astype('uint8')
    return Image.fromarray(arr,'RGB').convert('RGBA')


def glow(img,x,y,r,color=(251,189,91),power=140):
    layer=Image.new('RGBA',(W,H));p=ImageDraw.Draw(layer,'RGBA')
    for radius, alpha in [(r*2.3,6),(r*1.7,11),(r*1.2,24),(r*.65,45)]:
        p.ellipse((x-radius,y-radius,x+radius,y+radius),fill=(*color,alpha))
    layer=layer.filter(ImageFilter.GaussianBlur(max(3,r*.1)))
    img.alpha_composite(layer)
    p=ImageDraw.Draw(img,'RGBA')
    p.ellipse((x-3,y-3,x+3,y+3),fill=(*color,min(255,power+80)))


def dust(img,seed,amount=550,top=0,bottom=690):
    rr=random.Random(seed);d=ImageDraw.Draw(img,'RGBA')
    for _ in range(amount):
        x=rr.randrange(W); y=rr.randrange(top,bottom); r=rr.choice([.5,1,1,1.5,2]);
        c=rr.choice([(255,207,131,90),(207,225,255,100),(255,240,190,75)])
        d.ellipse((x-r,y-r,x+r,y+r),fill=c)


def starfield(img,seed,n=230):
    rr=random.Random(seed); d=ImageDraw.Draw(img,'RGBA')
    for _ in range(n):
        x=rr.randrange(W);y=rr.randrange(0,380);r=rr.choices([.6,1,1.6,2.2],[3,5,2,1])[0]
        d.ellipse((x-r,y-r,x+r,y+r),fill=(240,213,152,rr.randrange(50,190)))
    for _ in range(8):
        x=rr.randrange(30,1250);y=rr.randrange(20,320)
        d.line([(x-8,y),(x+8,y)],fill=(242,195,103,115),width=1)
        d.line([(x,y-8),(x,y+8)],fill=(242,195,103,115),width=1)


def window(d,x,y,w,h,lit=True):
    d.rounded_rectangle((x-3,y-3,x+w+3,y+h+4),radius=max(3,w//2),fill=(11,22,47,255),outline=(135,106,77,160),width=2)
    d.rounded_rectangle((x,y,x+w,y+h),radius=max(2,w//2),fill=((229,154,70,220) if lit else (40,66,95,220)))
    d.line((x+w/2,y+3,x+w/2,y+h-2),fill=(75,62,69,170),width=2)
    d.line((x+2,y+h*.55,x+w-2,y+h*.55),fill=(75,62,69,170),width=1)


def skyline(img,seed,baseline=510,height=170,palette=((12,22,46),(25,35,65))):
    rr=random.Random(seed); d=ImageDraw.Draw(img,'RGBA');x=-30
    while x<W+30:
        w=rr.randint(45,105);h=rr.randint(int(height*.55),height)
        top=baseline-h
        color=rr.choice(palette)
        d.polygon([(x,baseline),(x,top+15),(x+w/2,top-rr.randint(4,25)),(x+w,top+15),(x+w,baseline)],fill=(*color,250))
        d.rectangle((x+w*.42,top-25,x+w*.56,top+15),fill=(*color,240))
        for wy in range(top+35,baseline-15,29):
            for wx in range(x+12,x+w-12,19):
                if rr.random()<.64:window(d,int(wx),int(wy),7,12,rr.random()<.78)
        x+=w-3


def lamp(img,x,y,size=28,seed=0):
    d=ImageDraw.Draw(img,'RGBA')
    d.line((x,y-size*2.4,x,y-size*.6),fill=(172,128,71,255),width=4)
    d.ellipse((x-size*.5,y-size*.8,x+size*.5,y+size*.8),fill=(243,162,68,195),outline=(221,182,103,255),width=3)
    d.line((x-size*.6,y-size*.9,x+size*.6,y-size*.9),fill=(165,110,65,255),width=4)
    d.line((x-size*.65,y+size*.9,x+size*.65,y+size*.9),fill=(165,110,65,255),width=4)
    d.line((x,y-size*.7,x,y+size*.7),fill=(253,234,160,180),width=2)
    glow(img,x,y,size*1.8)


def rails(d,center=(640,550),bottom=H):
    cx,cy=center
    for offset in (-96,96):
        d.line([(cx+offset*.30,cy),(cx+offset*2.4,bottom)],fill=(34,27,44,255),width=19)
        d.line([(cx+offset*.30,cy),(cx+offset*2.4,bottom)],fill=(201,154,81,240),width=4)
    for t in np.linspace(.05,1,20):
        y=cy+int((bottom-cy)*t*t); half=40+int(200*t*t)
        d.line((cx-half,y,cx+half,y),fill=(70,47,52,230),width=max(1,int(8*t)))


def build_garden():
    im=gradient((5,12,35),(37,57,83),(21,40,58),31);d=ImageDraw.Draw(im,'RGBA');starfield(im,31)
    d.ellipse((865,72,1065,272),fill=(169,167,153,30),outline=(221,187,126,80),width=2)
    d.ellipse((905,105,1025,225),fill=(242,213,153,190))
    skyline(im,43,490,130,((11,24,48),(18,34,60)));d=ImageDraw.Draw(im,'RGBA')
    d.polygon([(0,555),(165,510),(380,540),(590,502),(880,545),(1280,506),(1280,720),(0,720)],fill=(20,41,52,255))
    # The garden has its own spiral of brass-marked plots.
    cx,cy=610,563; pts=[]
    for t in np.linspace(0,5*math.pi,550):
        r=19+t*17.1; x=cx+math.cos(t)*r; y=cy+math.sin(t)*r*.31
        pts.append((x,y))
    d.line(pts,fill=(132,112,82,245),width=32,joint='curve');d.line(pts,fill=(220,190,120,190),width=3,joint='curve')
    rr=random.Random(81)
    for i in range(48):
        t=i*.66;r=22+t*8.4;x=int(cx+math.cos(t)*r);y=int(cy+math.sin(t)*r*.30)
        if 35<x<1230 and 395<y<685:
            d.line((x,y-rr.randint(25,45),x,y),fill=(125,113,76,250),width=2)
            d.ellipse((x-6,y-rr.randint(38,53),x+6,y-26),fill=(193,148,78,230),outline=(246,204,131,240),width=2)
            if rr.random()<.4:glow(im,x,y-34,7,(239,182,105))
    # Wisteria in the foreground, with hanging violet clusters.
    d.polygon([(0,0),(0,475),(65,380),(137,255),(215,160),(280,90),(270,0)],fill=(8,23,35,255))
    d.polygon([(1280,0),(1280,470),(1195,355),(1122,245),(1040,155),(1005,0)],fill=(8,20,39,255))
    for side,seed in [(0,47),(1280,53)]:
        rr=random.Random(seed)
        for _ in range(110):
            x=int((rr.random()*250) if side==0 else (1280-rr.random()*250))
            y=int(rr.random()*360)
            col=rr.choice([(65,91,89,230),(61,82,73,220),(85,83,105,220)])
            d.ellipse((x-rr.randint(8,22),y-9,x+13,y+15),fill=col)
            if rr.random()<.3:
                d.line((x,y,x+rr.randint(-4,8),y+rr.randint(18,54)),fill=(119,103,82,190),width=1)
                for yy in range(y+15,y+34,7):d.ellipse((x-4,yy-3,x+5,yy+5),fill=(154,123,181,190))
    lamp(im,150,480,23);lamp(im,1120,456,25);dust(im,33,530,50,660)
    return im


def build_ministry():
    im=gradient((14,23,42),(73,66,74),(26,27,44),17);d=ImageDraw.Draw(im,'RGBA')
    d.rectangle((0,0,W,H),fill=(27,34,52,180))
    # Tall arch windows let in evening blue light; masonry vault and gilt frames.
    for x in (300,610,930):
        d.rounded_rectangle((x-92,45,x+92,515),radius=80,fill=(89,113,126,230),outline=(184,150,94,255),width=10)
        d.rounded_rectangle((x-70,60,x+70,500),radius=65,fill=(77,107,139,240),outline=(27,34,51,255),width=9)
        for yy in range(180,500,63):d.line((x-67,yy,x+67,yy),fill=(36,48,67,245),width=8)
        d.line((x,85,x,490),fill=(40,51,70,225),width=6)
        d.polygon([(x-64,485),(x+60,485),(x+200,715),(x-220,715)],fill=(230,182,115,24))
    for x in (120,460,770,1140):
        d.rectangle((x-27,0,x+27,615),fill=(31,34,49,250),outline=(163,117,72,250),width=5)
        for y in (40,70,555):d.line((x-38,y,x+38,y),fill=(204,153,82,185),width=5)
    for x in (25,1030):
        d.rectangle((x,74,x+220,555),fill=(27,24,35,255),outline=(170,121,71,250),width=8)
        rr=random.Random(x)
        for y in range(100,520,53):
            d.line((x+12,y+42,x+204,y+42),fill=(154,110,72,255),width=7)
            cur=x+20
            for _ in range(15):
                w=rr.randint(4,13);col=rr.choice([(62,87,110),(115,79,65),(151,123,83),(70,69,91),(175,123,71)])
                d.rectangle((cur,y+rr.randint(4,19),cur+w,y+40),fill=(*col,245))
                cur+=w+2
    d.polygon([(0,610),(W,575),(W,H),(0,H)],fill=(29,23,36,245))
    # Ministry counter, petitions, wax seals and bottle collection.
    d.polygon([(240,544),(965,537),(1130,622),(135,639)],fill=(97,65,52,255),outline=(201,149,88,255))
    d.polygon([(135,639),(1130,622),(1105,720),(120,720)],fill=(55,40,49,255),outline=(153,101,63,245))
    for x,y in [(360,553),(570,540),(740,568),(820,550)]:
        d.polygon([(x,y),(x+60,y-5),(x+67,y+10),(x+4,y+18)],fill=(222,198,161,245),outline=(119,86,63,220))
        for dy in range(2,15,5):d.line((x+7,y+dy,x+51,y+dy-4),fill=(99,91,87,95),width=1)
    for x in range(310,930,61):
        xx=x+int(14*math.sin(x)); y=492+int(11*math.sin(x/39))
        d.rectangle((xx-8,y-19,xx+8,y+18),fill=(68,107,110,170),outline=(187,155,108,245),width=2)
        d.ellipse((xx-8,y-27,xx+8,y-14),fill=(188,202,166,90))
    for x in (188,1090):lamp(im,x,356,20)
    dust(im,54,570,0,685);return im


def build_quay():
    im=gradient((7,15,38),(44,63,87),(20,39,63),61);starfield(im,62,190);d=ImageDraw.Draw(im,'RGBA')
    d.ellipse((865,58,1040,231),fill=(221,196,145,100))
    d.ellipse((886,75,1026,214),fill=(247,219,155,215))
    skyline(im,71,442,215,((16,35,66),(22,38,64),(10,25,47)));d=ImageDraw.Draw(im,'RGBA')
    # Water-level bridge and underwater doorways.
    d.rectangle((0,440,W,H),fill=(15,54,81,225));
    for y in range(453,712,21):
        alpha=int(105*(1-(y-440)/300))
        d.line((0,y,W,y+9),fill=(57,113,137,max(8,alpha)),width=2)
    for x in (105,310,512,785,998,1170):
        d.rounded_rectangle((x-48,488,x+48,603),radius=35,fill=(7,30,54,245),outline=(171,137,93,235),width=6)
        d.rounded_rectangle((x-33,499,x+33,594),radius=24,fill=(49,130,147,90))
        d.line((x-42,600,x+42,600),fill=(194,159,108,160),width=3)
        glow(im,x,558,18,(100,182,171))
    # Foreground dock narrows toward the old street-market.
    d.polygon([(0,525),(305,494),(500,522),(620,720),(0,720)],fill=(47,46,57,255),outline=(172,121,78,245))
    d.polygon([(W,509),(990,485),(840,519),(700,720),(W,720)],fill=(52,44,54,255),outline=(172,121,78,245))
    for y in (550,585,640,690):
        d.line((0,y,490+(y-510)*.55,y+3),fill=(174,139,90,155),width=3)
        d.line((W,y,790-(y-510)*.45,y+3),fill=(174,139,90,155),width=3)
    # Seller booth and warm glow on left quay.
    d.polygon([(35,375),(250,375),(280,402),(15,406)],fill=(115,70,72,255),outline=(207,155,103,245))
    d.line((35,375,35,544),fill=(177,127,76,255),width=8);d.line((253,375,253,535),fill=(177,127,76,255),width=8)
    for x in (64,112,162,211):lamp(im,x,417,11)
    # Floating memory tickets; no text embedded to avoid decorative fake UI.
    rr=random.Random(87)
    for _ in range(55):
        x=rr.randrange(0,W);y=rr.randrange(480,690);size=rr.randrange(3,14)
        d.ellipse((x-size,y-size//4,x+size,y+size//4),fill=(200,158,101,rr.randint(50,140)))
    lamp(im,342,510,29);lamp(im,1090,500,26);dust(im,61,450,15,640)
    return im


def build_hall():
    im=gradient((9,15,33),(45,50,74),(36,30,47),81);d=ImageDraw.Draw(im,'RGBA')
    d.rectangle((0,0,W,H),fill=(18,25,49,200));
    # Ribbed roof and immense central stained-glass rose window.
    for j in range(7):
        x=j*214-20
        d.arc((x-190,-175,x+190,380),170,360,fill=(168,123,73,200),width=7)
    d.ellipse((466,32,818,384),fill=(35,52,77,255),outline=(208,166,98,255),width=14)
    d.ellipse((485,51,799,365),fill=(37,83,105,255),outline=(178,145,92,250),width=4)
    for j in range(24):
        a=math.pi*2*j/24
        x1,y1=642+28*math.cos(a),207+28*math.sin(a)
        x2,y2=642+155*math.cos(a),207+155*math.sin(a)
        d.line((x1,y1,x2,y2),fill=(208,151,83,220),width=4)
        r=30
        d.ellipse((x2-r/3,y2-r/3,x2+r/3,y2+r/3),fill=(141,109,179,150))
    d.ellipse((603,168,681,246),fill=(233,190,113,230),outline=(245,220,161,255),width=4)
    for x in (160,1100):
        d.rounded_rectangle((x-66,132,x+66,462),radius=60,fill=(33,91,99,220),outline=(199,152,92,255),width=10)
        for y in range(190,450,65):d.line((x-55,y,x+55,y),fill=(206,161,91,160),width=3)
    for x in (40,330,940,1240):
        d.rectangle((x-26,0,x+26,590),fill=(24,31,54,255),outline=(162,119,72,240),width=5)
        d.rectangle((x-36,530,x+36,563),fill=(94,67,61,255))
    d.polygon([(0,590),(W,580),(W,H),(0,H)],fill=(49,38,52,255))
    d.polygon([(486,470),(812,470),(885,570),(393,570)],fill=(110,66,62,250),outline=(232,174,95,255))
    d.rectangle((520,405,780,471),fill=(77,48,53,240),outline=(193,156,105,245),width=8)
    # Rows of citizen benches (not perfectly symmetric).
    for side in (-1,1):
        for y in (550,609,677):
            extent=140+int((y-520)*1.3)
            center=640+side*(extent+118)
            d.polygon([(center-140,y),(center+140,y),(center+173,y+15),(center-164,y+15)],fill=(102,68,62,255),outline=(199,150,88,255))
            d.line((center-135,y+20,center+139,y+20),fill=(43,30,41,220),width=17)
    for x,y in [(235,328),(1045,334),(420,405),(864,405)]:lamp(im,x,y,20)
    for x,y in [(556,472),(612,480),(686,480),(731,472)]:
        d.polygon([(x,y),(x+21,y-5),(x+19,y+7),(x,y+9)],fill=(228,210,172,235))
    dust(im,112,640,0,685)
    return im


def build_station():
    im=gradient((11,19,41),(45,51,72),(22,30,45),21);d=ImageDraw.Draw(im,'RGBA')
    d.rectangle((0,0,W,H),fill=(16,24,42,145))
    # Iron-vaulted platform, open sky beyond tracks.
    d.rounded_rectangle((330,115,945,590),radius=180,fill=(50,79,108,255),outline=(190,143,85,255),width=14)
    d.rounded_rectangle((360,136,920,571),radius=170,fill=(28,49,79,255))
    skyline(im,74,570,145,((15,31,55),(20,33,56)));d=ImageDraw.Draw(im,'RGBA')
    # Perspective roof struts and empty clock faces.
    for xx in (5,165,325,955,1120,1275):
        d.polygon([(xx-30,0),(xx+30,0),(640+(xx-640)*.42,472),(640+(xx-640)*.39,466)],fill=(49,53,66,255),outline=(171,128,77,230))
        d.line((xx,0,640+(xx-640)*.4,469),fill=(205,156,96,185),width=3)
    for x in (160,1110):
        d.ellipse((x-95,75,x+95,265),fill=(43,52,67,255),outline=(225,177,106,255),width=11)
        d.ellipse((x-70,101,x+70,241),fill=(119,129,130,195),outline=(35,33,54,250),width=7)
        for k in range(12):
            a=math.pi*2*k/12
            d.line((x+60*math.cos(a),171+60*math.sin(a),x+69*math.cos(a),171+69*math.sin(a)),fill=(202,155,93,230),width=3)
    d.polygon([(0,565),(W,553),(W,H),(0,H)],fill=(30,35,49,255))
    rails(d,center=(640,548))
    # Timetable plates and unclaimed tickets suspended like leaves.
    for x in (56,1032):
        d.rectangle((x,289,x+194,430),fill=(37,41,58,255),outline=(203,153,90,250),width=7)
        for i in range(8):
            y=307+i*14;d.line((x+15,y,x+164-13*(i%4),y),fill=(163,141,106,130),width=2)
    rr=random.Random(66)
    for _ in range(55):
        x=rr.randint(355,920);y=rr.randint(360,540)
        if rr.random()<.6:
            d.line((x,y-30,x,y),fill=(142,110,78,110),width=1)
            d.rectangle((x-8,y,x+8,y+16),fill=(209,194,152,rr.randrange(100,235)),outline=(171,142,102,180),width=1)
    for x in (235,1040):lamp(im,x,452,27)
    dust(im,144,530,60,700)
    return im


BUILDERS={
    'bell_garden':build_garden,
    'relief_ministry':build_ministry,
    'unreturned_quay':build_quay,
    'open_questions_hall':build_hall,
    'forgotten_station':build_station,
}

def main():
    OUT.mkdir(parents=True,exist_ok=True)
    for name,fn in BUILDERS.items():
        path=OUT/(name+'.png');fn().convert('RGB').save(path,optimize=True)
        print(path,path.stat().st_size)

if __name__=='__main__':main()
