#!/usr/bin/env python3
"""Original deterministic illustrated environment for Atlas's Glass Ferry chapter.

Build-time artwork authoring only; no dependency in the player or engine.
"""
import math, random
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageEnhance

SEED=30451
rng=random.Random(SEED)
S=2; W=1280*S; H=720*S
im=Image.new('RGB',(W,H)); a=np.zeros((H,W,3),dtype=np.uint8)
# water-colour sky: deep indigo overhead, rose/lavender horizon
stops=[(0,(6,12,37)),(.24,(13,25,65)),(.49,(38,53,100)),(.65,(91,83,117)),(.71,(184,118,106)),(1,(9,23,52))]
for y in range(H):
 t=y/(H-1); k=next(i for i in range(len(stops)-1) if stops[i][0]<=t<=stops[i+1][0]);u=(t-stops[k][0])/(stops[k+1][0]-stops[k][0]);
 col=[int(stops[k][1][j]*(1-u)+stops[k+1][1][j]*u) for j in range(3)];
 a[y,:,:]=col
# subtle diagonal lighting and radial twilight
xx=np.linspace(-1,1,W,dtype=np.float32)
for y in range(H):
 wy=y/H
 aura=(np.exp(-((xx+.17)/.60)**2-((wy-.59)/.20)**2)*30 + np.exp(-((xx-.62)/.32)**2-((wy-.48)/.20)**2)*12)
 a[y,:,0]=np.clip(a[y,:,0].astype(np.float32)+aura,0,255)
 a[y,:,1]=np.clip(a[y,:,1].astype(np.float32)+aura*.48,0,255)
 a[y,:,2]=np.clip(a[y,:,2].astype(np.float32)+aura*.57,0,255)
im=Image.fromarray(a,'RGB').convert('RGBA')

# atmospheric luminous cloudscape, with transparent, blurred layers
clouds=Image.new('RGBA',(W,H));cd=ImageDraw.Draw(clouds,'RGBA')
for _ in range(530):
 x=rng.randrange(-100*S,1350*S);y=rng.randrange(130*S,500*S)
 spread=rng.randrange(30*S,215*S);h=rng.randrange(7*S,42*S)
 col=rng.choice([(81,98,166,13),(158,135,190,12),(244,168,135,13),(32,58,116,29)])
 cd.ellipse((x-spread//2,y-h,x+spread//2,y+h), fill=col)
clouds=clouds.filter(ImageFilter.GaussianBlur(24*S))
im=Image.alpha_composite(im,clouds)

# starfield, distant and near: constellations & luminous arcs
star_layer=Image.new('RGBA',(W,H));sd=ImageDraw.Draw(star_layer,'RGBA')
for _ in range(700):
 x=rng.randrange(W);y=rng.randrange(0,420*S)
 p=rng.random();r=(.25 if p<.78 else .55 if p<.94 else 1.25)*S
 c=(185,205,250,100) if p<.8 else (247,224,165,200)
 sd.ellipse((x-r,y-r,x+r,y+r),fill=c)
for (x,y,r) in [(438,110,2.6),(512,147,2.3),(601,125,3.3),(668,183,2.1),(741,168,3.0),(784,118,2.7),(902,92,3.6)]:
 x*=S;y*=S;r*=S
 sd.line((x-8*S,y,x+8*S,y),fill=(252,216,138,190),width=S)
 sd.line((x,y-8*S,x,y+8*S),fill=(252,216,138,190),width=S)
 sd.ellipse((x-r,y-r,x+r,y+r),fill=(255,244,209,255))
# a few fine celestial paths, separated from architectural silhouettes
for k in range(3):
 pts=[]
 for x in range(240,975,6):
  y=128+27*k+49*math.sin(x/186+0.9*k)
  pts.append((x*S,int(y*S)))
 sd.line(pts,fill=(205,185,146,28),width=1*S)
im=Image.alpha_composite(im,star_layer)

# crescent moon, high over the far shore
moon=Image.new('RGBA',(W,H));md=ImageDraw.Draw(moon,'RGBA')
mx,my=1070*S,108*S
for rr,alpha in [(72,15),(55,26),(42,40)]:md.ellipse((mx-rr*S,my-rr*S,mx+rr*S,my+rr*S),fill=(148,180,241,alpha))
md.ellipse((mx-30*S,my-30*S,mx+30*S,my+30*S),fill=(229,228,209,215))
md.ellipse((mx-18*S,my-39*S,mx+43*S,my+27*S),fill=(13,24,57,255))
moon=moon.filter(ImageFilter.GaussianBlur(S*.6));im=Image.alpha_composite(im,moon)

# Distant city silhouettes: midnight spires and copper roofs
city=Image.new('RGBA',(W,H));d=ImageDraw.Draw(city,'RGBA')
horizon=438*S
for side in [0,1]:
 for i in range(63):
  x=rng.randint(0,370) if side==0 else rng.randint(950,1280)
  ybase=470+rng.randint(-19,12);ht=rng.randint(25,126);width=rng.randint(8,29)
  x*=S;ybase*=S;ht*=S;width*=S
  shade=(12,24,45,255) if side==0 else (13,27,46,255)
  d.rectangle((x,ybase-ht,x+width,ybase),fill=shade)
  d.polygon((x-4*S,ybase-ht,x+width//2,ybase-ht-rng.randint(9,52)*S,x+width+4*S,ybase-ht),fill=(12,22,44,255))
  if i%4==0:
   d.line((x+width//2,ybase-ht-8*S,x+width//2,ybase-ht-40*S),fill=(42,47,72,220),width=S)
  for wy in range(ybase-ht+6*S,ybase-6*S,8*S):
   if rng.random()<.68:
    wx=x+rng.randint(2,max(3,width//S-3))*S
    d.ellipse((wx,wy,wx+2*S,wy+3*S),fill=(242,176,91,rng.randint(160,235)))
# towers close on side of quay
for x,y,w,h in [(50,475,84,234),(160,474,52,154),(1110,465,78,212),(1220,470,53,284)]:
 x*=S;y*=S;w*=S;h*=S
 d.polygon((x-8*S,y-h,x+w//2,y-h-45*S,x+w+8*S,y-h,x+w+8*S,y),fill=(17,25,43,255))
 d.rectangle((x,y-h,x+w,y),fill=(19,30,49,255))
 d.rectangle((x+6*S,y-h+17*S,x+w-6*S,y-h+20*S),fill=(144,98,65,225))
 for row in range(2,11):
  for col in range(2):
   wx=x+(18+col*36)*S;wy=y-h+(20+row*17)*S
   if wy<y-8*S:d.rounded_rectangle((wx,wy,wx+6*S,wy+9*S),radius=2*S,fill=(211,151,72,235))
city=city.filter(ImageFilter.GaussianBlur(.42*S));im=Image.alpha_composite(im,city)

# Blue glass canal with mirrored glint and etched navigation lines
water=Image.new('RGBA',(W,H));wd=ImageDraw.Draw(water,'RGBA')
wd.polygon([(0,469*S),(W,460*S),(W,H),(0,H)],fill=(6,29,66,236))
# perspective channels converge far into the city
for i in range(42):
 fy=467+int(i*6.1);dy=(fy-460)/260
 span=int(57+520*dy)
 for direction in [-1,1]:
  cx=675*S+direction*rng.randint(5,max(20,span))*S
  length=rng.randint(20,140)*S
  color=rng.choice([(64,122,189,110),(134,181,221,72),(237,172,98,90),(74,179,223,100)])
  wd.line((cx-length//2,fy*S,cx+length//2,fy*S),fill=color,width=rng.randint(1,2)*S)
# drifting water stars and copper constellations under the surface
for j in range(260):
 x=rng.randint(90,1180)*S;y=rng.randint(475,704)*S
 rr=(y//S-460)/260
 r=max(1,int(rr*1.9*S));wd.ellipse((x-r,y-r,x+r,y+r),fill=(132,215,243,rng.randint(50,165)))
# a glowing passage from ferry to illuminated distant shore
for j in range(20):
 y=(468+j*11)*S;width=(5+2*j)*S
 wd.line((672*S-width,y,672*S+width,y),fill=(105,190,231,max(30,155-j*6)),width=S)
im=Image.alpha_composite(im,water)

# canal mist and drifting mirrored curtains
mist=Image.new('RGBA',(W,H));mistd=ImageDraw.Draw(mist,'RGBA')
for i in range(180):
 x=rng.randrange(W);y=rng.randrange(405*S,550*S)
 r=rng.randrange(20*S,140*S)
 mistd.ellipse((x-r,y-8*S,x+r,y+6*S),fill=(148,171,213,rng.randrange(5,22)))
mist=mist.filter(ImageFilter.GaussianBlur(17*S));im=Image.alpha_composite(im,mist)

# Stone docks and iron railings under distant lanterns
fore=Image.new('RGBA',(W,H));f=ImageDraw.Draw(fore,'RGBA')
f.polygon([(0,533*S),(405*S,517*S),(490*S,580*S),(190*S,H),(0,H)],fill=(16,23,36,255))
f.polygon([(1070*S,514*S),(W,517*S),(W,H),(1035*S,620*S)],fill=(14,20,35,255))
f.line([(0,533*S),(405*S,517*S),(490*S,580*S)],fill=(151,100,59,210),width=4*S)
f.line([(1070*S,514*S),(W,517*S)],fill=(173,116,65,211),width=5*S)
for x,y in [(44,540),(167,535),(305,530),(1125,519),(1201,518)]:
 x*=S;y*=S
 f.rectangle((x-6*S,y-12*S,x+6*S,y+61*S),fill=(27,28,39,255))
 f.ellipse((x-8*S,y-15*S,x+8*S,y-4*S),fill=(164,104,61,255))
# silhouettes of stacked packing cases and rope coils
for x,y,w,h in [(30,599,95,50),(150,615,115,62),(1140,604,68,52)]:
 f.polygon([(x*S,y*S),((x+w)*S,(y-8)*S),((x+w)*S,(y+h-8)*S),(x*S,(y+h)*S)],fill=(52,40,40,255))
 f.line((x*S,y*S,(x+w)*S,(y-8)*S),fill=(152,109,65,230),width=3*S)

# Ferry in three-quarter profile: gold-green cabin with glass windows
hull=[(445,589),(553,573),(832,567),(1003,592),(944,633),(507,647)]
f.polygon([(x*S,y*S) for x,y in hull],fill=(20,37,54,255))
f.line([(x*S,y*S) for x,y in hull[:4]],fill=(212,157,85,250),width=5*S)
f.polygon([(520*S,595*S),(970*S,594*S),(930*S,620*S),(553*S,629*S)],fill=(37,66,84,250))
f.line((540*S,622*S,926*S,614*S),fill=(114,176,176,175),width=3*S)
# brass ribs and forward bow insignia
for x in (567,627,690,753,821,885):
 f.line((x*S,594*S,x*S,626*S),fill=(196,151,79,210),width=3*S)
f.ellipse((949*S,596*S,961*S,608*S),outline=(252,198,113,255),width=3*S)
# raised cabin with sloping copper roof
f.rounded_rectangle((625*S,475*S,838*S,570*S),radius=9*S,fill=(28,65,70,252),outline=(171,118,63,255),width=4*S)
f.polygon([(610*S,485*S),(634*S,455*S),(835*S,449*S),(863*S,482*S)],fill=(65,65,59,255))
f.line((615*S,482*S,861*S,478*S),fill=(228,158,80,240),width=4*S)
for i in range(3):
 x=(643+i*62)*S
 f.rounded_rectangle((x,497*S,x+47*S,556*S),radius=7*S,fill=(217,156,88,254),outline=(205,194,134,240),width=3*S)
 f.rectangle((x+6*S,504*S,x+41*S,549*S),fill=(22,66,76,215))
 f.line((x+23*S,504*S,x+23*S,549*S),fill=(207,181,110,215),width=2*S)
 # warm backlit silhouettes in cabin
 f.ellipse((x+18*S,518*S,x+27*S,528*S),fill=(230,160,98,180))
 f.ellipse((x+15*S,526*S,x+30*S,548*S),fill=(63,58,54,220))
# upper deck ropes and decorative wheel
f.line((475*S,577*S,603*S,556*S),fill=(177,132,80,225),width=3*S)
f.line((845*S,561*S,985*S,584*S),fill=(177,132,80,225),width=3*S)
for x,y in [(508,567),(569,557),(864,561),(922,575)]:
 f.ellipse(((x-4)*S,(y-4)*S,(x+4)*S,(y+4)*S),fill=(235,184,89,255))
f.ellipse((560*S,545*S,600*S,585*S),outline=(213,167,96,245),width=3*S)
for i in range(8):
 theta=i*math.pi/4
 f.line((580*S,565*S,(580+18*math.cos(theta))*S,(565+18*math.sin(theta))*S),fill=(188,149,81,255),width=2*S)
# boom & suspended lantern
f.line((777*S,465*S,777*S,382*S),fill=(149,122,84,255),width=4*S)
f.line((778*S,385*S,823*S,387*S),fill=(168,134,78,255),width=2*S)
f.line((817*S,387*S,817*S,417*S),fill=(183,141,90,255),width=2*S)
f.polygon([(806*S,420*S),(828*S,420*S),(834*S,446*S),(800*S,446*S)],fill=(235,161,77,252))
f.line((801*S,438*S,833*S,438*S),fill=(252,221,122,255),width=3*S)
# graceful pedestrian on deck, head and cloak silhouette
f.ellipse((548*S,493*S,569*S,517*S),fill=(25,25,40,255))
f.polygon([(551*S,514*S),(568*S,516*S),(585*S,573*S),(530*S,575*S)],fill=(30,27,45,255))
f.line((538*S,571*S,519*S,585*S),fill=(30,27,45,255),width=6*S)
f.line((580*S,569*S,595*S,581*S),fill=(30,27,45,255),width=5*S)
# figure's copper compass brooch
f.ellipse((553*S,526*S,561*S,534*S),fill=(255,184,79,245))
im=Image.alpha_composite(im,fore)

# lantern bloom over ferry and city lights; soft gold, localized
lights=Image.new('RGBA',(W,H));ld=ImageDraw.Draw(lights,'RGBA')
for x,y,r in [(817,437,62),(670,517,30),(150,413,37),(1132,353,36)]:
 cx=x*S;cy=y*S;rr=r*S
 for mult,al in [(1.8,16),(1.15,28),(.7,48),(.35,90)]:
  rr2=rr*mult
  ld.ellipse((cx-rr2,cy-rr2,cx+rr2,cy+rr2),fill=(255,164,61,al))
lights=lights.filter(ImageFilter.GaussianBlur(14*S));im=Image.alpha_composite(im,lights)

# foreground etched navigation motif: high-quality decorative story detail
orn=Image.new('RGBA',(W,H));od=ImageDraw.Draw(orn,'RGBA')
# left brass compass half visible on the dock
cx,cy=114*S,657*S
for rr,alpha in [(48,120),(43,140),(31,85)]:
 od.ellipse((cx-rr*S,cy-rr*S,cx+rr*S,cy+rr*S),outline=(228,173,91,alpha),width=2*S)
for ang in range(0,360,30):
 t=math.radians(ang)
 od.line((cx+36*S*math.cos(t),cy+36*S*math.sin(t),cx+47*S*math.cos(t),cy+47*S*math.sin(t)),fill=(232,185,106,145),width=S)
od.polygon([(cx,cy-40*S),(cx+8*S,cy),(cx,cy+40*S),(cx-8*S,cy)],outline=(240,188,117,185))
im=Image.alpha_composite(im,orn)

# painted fine-grain texture and final filmic contrast
im=im.convert('RGB');arr=np.asarray(im,dtype=np.float32).copy()
rg=np.random.default_rng(SEED)
noise=rg.normal(0,2.4,(H,W,1)).astype(np.float32)
arr=np.clip(arr+noise,0,255).astype(np.uint8)
im=Image.fromarray(arr,'RGB').resize((1280,720),Image.Resampling.LANCZOS)
im=ImageEnhance.Color(im).enhance(1.06)
root=Path(__file__).resolve().parents[1]
final=root / 'assets' / 'backgrounds' / 'blue_glass_ferry.png'
final.parent.mkdir(parents=True, exist_ok=True)
# Stable, compact 1280x720 palette for Vesper's external game assets.
pal=im.quantize(colors=240,method=Image.Quantize.MEDIANCUT,dither=Image.Dither.FLOYDSTEINBERG)
pal.save(final,optimize=True)
print('Rendered', final, final.stat().st_size)

