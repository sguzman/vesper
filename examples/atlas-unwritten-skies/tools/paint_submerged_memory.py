#!/usr/bin/env python3
"""Original illustrated submerged-memory chamber; build-time media tool only."""
from pathlib import Path
import math, random
import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageEnhance
rng=random.Random(71948)
S=2; W=1280*S; H=720*S
# underwater celestial gradient with etched light
arr=np.zeros((H,W,3),dtype=np.uint8)
for y in range(H):
 t=y/H
 for j,(u,v) in enumerate([(8,30),(39,72),(77,96)]):
  arr[y,:,j]=max(0,min(255,int((1-t)*u+t*v)))
x=np.linspace(-1,1,W)
for y in range(H):
 yy=y/H
 glow=68*np.exp(-((x-.18)/.48)**2-((yy-.29)/.38)**2)+31*np.exp(-((x+.45)/.59)**2-((yy-.3)/.36)**2)
 arr[y,:,0]=np.clip(arr[y,:,0].astype(float)+glow*.38,0,255)
 arr[y,:,1]=np.clip(arr[y,:,1].astype(float)+glow*.87,0,255)
 arr[y,:,2]=np.clip(arr[y,:,2].astype(float)+glow,0,255)
im=Image.fromarray(arr,'RGB').convert('RGBA')

# spectral motes and aquatic shafts
rays=Image.new('RGBA',(W,H));d=ImageDraw.Draw(rays,'RGBA')
for j in range(8):
 top=(720+j*67)*S;spread=(120+j*21)*S
 d.polygon([(top,0),(top+spread//3,0),(top+spread,630*S),(top-30*S,630*S)],fill=(111,209,215,6+j%2*4))
rays=rays.filter(ImageFilter.GaussianBlur(37*S));im=Image.alpha_composite(im,rays)

# stone hall: arches, distant staircase and ribs of a drowned observatory
arch=Image.new('RGBA',(W,H));d=ImageDraw.Draw(arch,'RGBA')
for x0 in [0,240,485,790,1090]:
 x=x0*S
 d.rectangle((x,0,x+24*S,H),fill=(8,32,47,245))
 d.rectangle((x+4*S,0,x+16*S,H),fill=(23,66,73,200))
 d.ellipse((x-28*S,-104*S,x+216*S,202*S),outline=(92,127,128,95),width=8*S)
 # gold seam along columns
 d.line((x+17*S,0,x+17*S,720*S),fill=(167,131,77,110),width=3*S)
d.polygon([(0,645*S),(370*S,577*S),(660*S,604*S),(990*S,569*S),(W,643*S),(W,H),(0,H)],fill=(10,34,49,255))
for z in range(11):
 p=(z/11)**1.5
 yy=(573+136*p)*S
 d.line((0,yy,W,yy),fill=(62,114,118,max(25,int(119-70*p))),width=S)
# stairs at left, diagonal receding
for i in range(12):
 y=(342+i*28)*S
 ex=(252+i*13)*S
 d.polygon([(0,y),(ex,y),(ex+16*S,y+12*S),(0,y+12*S)],fill=(24+i%3*4,68+i%2*5,72+i%2*4,235))
 d.line((0,y,ex,y),fill=(190,147,91,150),width=2*S)
im=Image.alpha_composite(im,arch)

# remote doorway with rays and a mysterious shore
remote=Image.new('RGBA',(W,H));d=ImageDraw.Draw(remote,'RGBA')
for i in range(17):
 r=(78+i*8)*S
 d.ellipse((655*S-r,342*S-r,655*S+r,342*S+r),outline=(99,176,181,max(3,55-3*i)),width=2*S)
d.rounded_rectangle((585*S,194*S,725*S,513*S),radius=65*S,fill=(44,89,95,115),outline=(183,153,93,209),width=5*S)
# distant light as doorway glass
for j in range(31):
 xx=(599+j*3)*S
 d.line((xx,224*S,xx,481*S),fill=(126,208,203,10),width=3*S)
im=Image.alpha_composite(im,remote)

# central luminous brass astrolabe rings suspended in water
rings=Image.new('RGBA',(W,H));d=ImageDraw.Draw(rings,'RGBA')
cx,cy=666*S,330*S
for r,alpha,width in [(174,82,2),(152,165,4),(138,100,2),(118,148,3),(88,178,2)]:
 rr=r*S;d.ellipse((cx-rr,cy-rr,cx+rr,cy+rr),outline=(236,184,101,alpha),width=width*S)
for theta in range(0,360,15):
 angle=math.radians(theta)
 a,b=135*S,159*S
 d.line((cx+a*math.cos(angle),cy+a*math.sin(angle),cx+b*math.cos(angle),cy+b*math.sin(angle)),fill=(234,188,99,135),width=2*S)
for i in range(7):
 t=i*math.pi/7
 d.arc((cx-169*S,cy-(170-18*i)*S,cx+169*S,cy+(170-18*i)*S),math.degrees(t),math.degrees(t)+160,fill=(132,230,222,73),width=2*S)
# center rotating eight pointed compass rose
points=[]
for j in range(16):
 t=j*math.pi/8;rr=(55 if j%2==0 else 17)*S
 points.append((cx+rr*math.sin(t),cy-rr*math.cos(t)))
d.polygon(points,fill=(173,137,84,190),outline=(248,210,125,210))
d.ellipse((cx-10*S,cy-10*S,cx+10*S,cy+10*S),fill=(235,214,159,255))
# suspended clamps and old instrument frame
for side in [-1,1]:
 x0=cx+side*166*S
 d.line((x0,100*S,x0,565*S),fill=(180,137,79,105),width=3*S)
d.line((cx,50*S,cx,146*S),fill=(163,130,78,174),width=4*S)
im=Image.alpha_composite(im,rings)

# ancient dining table and place settings below water
furn=Image.new('RGBA',(W,H));d=ImageDraw.Draw(furn,'RGBA')
d.ellipse((420*S,578*S,910*S,676*S),fill=(13,32,39,255))
d.polygon([(455*S,571*S),(846*S,567*S),(925*S,604*S),(394*S,610*S)],fill=(65,56,52,255),outline=(198,143,82,208))
d.line((420*S,604*S,909*S,600*S),fill=(230,176,96,205),width=5*S)
d.line((493*S,615*S,474*S,694*S),fill=(46,54,57,255),width=19*S)
d.line((853*S,614*S,867*S,698*S),fill=(49,56,55,255),width=19*S)
for px,py in [(498,591),(606,592),(714,590),(814,591)]:
 d.ellipse(((px-29)*S,(py-9)*S,(px+29)*S,(py+11)*S),fill=(38,68,72,240),outline=(220,178,102,230),width=2*S)
 d.ellipse(((px-13)*S,(py-4)*S,(px+13)*S,(py+6)*S),fill=(94,129,122,178))
# translucent chairs and wrought iron legs
for x0 in [376,949]:
 d.rounded_rectangle(((x0-17)*S,512*S,(x0+17)*S,626*S),radius=9*S,outline=(173,134,82,160),width=5*S)
 d.line((x0*S,595*S,(x0-9)*S,689*S),fill=(81,95,81,160),width=6*S)
im=Image.alpha_composite(im,furn)

# chandelier with burning candles beneath water
light=Image.new('RGBA',(W,H));d=ImageDraw.Draw(light,'RGBA')
chx,chy=1010*S,245*S
d.line((chx,0,chx,200*S),fill=(176,139,95,224),width=5*S)
for rr in [50,84]:
 d.arc((chx-rr*S,chy-27*S,chx+rr*S,chy+25*S),0,180,fill=(204,158,96,215),width=4*S)
for ox in [-72,-44,0,44,72]:
 xx=chx+ox*S; yy=(249+15*(abs(ox)/72))*S
 d.rectangle((xx-4*S,yy-27*S,xx+4*S,yy),fill=(229,212,159,232))
 d.ellipse((xx-5*S,yy-43*S,xx+5*S,yy-24*S),fill=(250,198,96,233))
 for rr,opacity in [(21,45),(34,20),(48,8)]:
  d.ellipse((xx-rr*S,yy-(32+rr)*S,xx+rr*S,yy-(32-rr)*S),fill=(245,203,121,opacity))
light=light.filter(ImageFilter.GaussianBlur(.8*S));im=Image.alpha_composite(im,light)

# glowing pale fish, dust, paper fragments
fish=Image.new('RGBA',(W,H));d=ImageDraw.Draw(fish,'RGBA')
for i in range(135):
 x=rng.randint(100,1200)*S;y=rng.randint(70,618)*S;r=rng.randint(2,5)*S
 col=(158,229,211,rng.randint(65,190))
 d.ellipse((x-r,y-r//2,x+r,y+r//2),fill=col)
 d.polygon([(x-r,y),(x-2*r,y-r//2),(x-2*r,y+r//2)],fill=col)
for i in range(1500):
 x=rng.randrange(W);y=rng.randrange(H);rr=rng.randint(1,2)*S
 d.ellipse((x,y,x+rr,y+rr),fill=(165,227,215,rng.randint(17,60)))
# drifting handwritten pages rendered as tiny unlettered parchment shapes
for i in range(17):
 x=rng.randint(140,1100)*S;y=rng.randint(130,480)*S
 if abs(x-cx)<180*S and abs(y-cy)<160*S:continue
 d.polygon([(x,y),(x+13*S,y-4*S),(x+15*S,y+15*S),(x+3*S,y+18*S)],fill=(179,194,172,45),outline=(218,190,127,75))
im=Image.alpha_composite(im,fish)

# water refractions across the whole scene
refract=Image.new('RGBA',(W,H));d=ImageDraw.Draw(refract,'RGBA')
for i in range(40):
 x=rng.randrange(-100,1350)*S;y=rng.randrange(0,720)*S
 length=rng.randrange(30,180)*S
 pts=[]
 for k in range(19):
  xx=x+k*length/18;yy=y+7*S*math.sin(k*.5+i)
  pts.append((xx,yy))
 d.line(pts,fill=(115,202,198,rng.randrange(12,38)),width=S)
refract=refract.filter(ImageFilter.GaussianBlur(1.3*S));im=Image.alpha_composite(im,refract)
# film grain and luminous layering
arr=np.asarray(im.convert('RGB'),dtype=np.float32).copy()
noise=np.random.default_rng(71948).normal(0,2.4,(H,W,1))
arr=np.clip(arr+noise,0,255).astype(np.uint8)
im=Image.fromarray(arr).resize((1280,720),Image.Resampling.LANCZOS)
im=ImageEnhance.Color(im).enhance(1.05)
final=Path(__file__).resolve().parents[1]/'assets/backgrounds/submerged_memory_chamber.png'
final.parent.mkdir(parents=True,exist_ok=True)
im.quantize(colors=240,method=Image.Quantize.MEDIANCUT,dither=Image.Dither.FLOYDSTEINBERG).save(final,optimize=True)
print('Rendered',final,final.stat().st_size)
