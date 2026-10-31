#!/usr/bin/env python3
"""Original, deterministic evolving ambient score for Atlas.

Offline content-authoring only. The player loads normal PCM WAV files; it never
runs Python or synthesizes audio. All melodies and sound-design are original.
"""
from pathlib import Path
import math, wave
import numpy as np

SAMPLE_RATE = 16000
SECONDS = 68
DEST = Path(__file__).resolve().parents[1] / 'assets/audio'

# (key root Hz, semitone scale, progression as degree offsets, speed, mood)
PIECES = {
    'archive': (174.61, [0,2,3,5,7,9,10,12], [0,5,3,7,0,3], 5.0, 'warm'),
    'threshold': (164.81, [0,1,3,5,7,8,10,12], [0,8,5,7,3,8], 4.4, 'mystic'),
    'mystery': (146.83, [0,1,3,6,7,10,11,12], [0,6,10,3,1,7], 5.6, 'uneasy'),
    'memory': (164.81, [0,2,3,5,7,8,10,12], [0,8,5,3,0,5], 6.1, 'gentle'),
    'journey': (196.00, [0,2,4,5,7,9,11,12], [0,5,7,9,5,7], 4.2, 'wide'),
    'storm': (130.81, [0,1,3,6,7,8,10,12], [0,6,1,7,3,6], 3.6, 'storm'),
    'dawn': (196.00, [0,2,4,7,9,11,12,14], [0,5,9,7,4,0], 5.8, 'clear'),
}

N = SECONDS * SAMPLE_RATE
TIMES = np.arange(N, dtype=np.float64) / SAMPLE_RATE


def hz(base: float, semitones: float) -> float:
    return base * 2.0 ** (semitones / 12.0)


def lp_noise(rng: np.random.Generator, size: int, blur: int) -> np.ndarray:
    noise = rng.normal(0, 1, size + blur - 1)
    # Cumulative-window box filter rather than expensive convolution.
    cs = np.cumsum(np.concatenate(([0.0], noise)))
    return (cs[blur:] - cs[:-blur]) / math.sqrt(blur)


def instrument(freq: float, rel_time: np.ndarray, flavor: str) -> np.ndarray:
    """Band-limited analog-like pad/bell timbre; no samples or interpolation."""
    phase = 2 * np.pi * freq * rel_time
    if flavor == 'bell':
        return (.65*np.sin(phase) + .25*np.sin(phase*2.011) +
                .11*np.sin(phase*3.931) + .055*np.sin(phase*5.19))
    if flavor == 'air':
        return (.77*np.sin(phase) + .20*np.sin(phase*2.002) +
                .065*np.sin(phase*3.01))
    return (.76*np.sin(phase) + .20*np.sin(phase*2) + .055*np.sin(phase*3))


def place_bell(buffer: np.ndarray, freq: float, start: float, dur: float, gain: float, rng,
               flavor='bell') -> None:
    st = int(start * SAMPLE_RATE)
    if st >= N:
        return
    size = min(N-st, max(1, int(dur * SAMPLE_RATE)))
    t = np.arange(size, dtype=np.float64) / SAMPLE_RATE
    attack = np.minimum(1, t/.018)
    env = attack * (np.exp(-t / max(.2, dur*.32)))
    vibrato = 1 + .0011*np.sin(2*np.pi*(4.1+rng.random())*t)
    harmonic = instrument(freq, t*vibrato, flavor)
    buffer[st:st+size] += harmonic * env * gain


def score(name: str, base: float, scale: list[int], chords: list[int], period: float, mood: str):
    rng = np.random.default_rng(500+sum(ord(c) for c in name))
    audio = np.zeros(N, dtype=np.float64)
    # Six expanding or contracting harmonic moments over one long loop.
    part = SECONDS / len(chords)
    pad = np.zeros(N, dtype=np.float64)
    for i, root in enumerate(chords):
        st = round(i * part * SAMPLE_RATE)
        en = min(N, round((i + 1) * part * SAMPLE_RATE)+int(2.5*SAMPLE_RATE))
        t = TIMES[st:en]
        offset = t - st / SAMPLE_RATE
        # Long fade-in/out per chord; overlapping tones blend beneath melody.
        att = np.clip(offset / 2.1, 0, 1)
        release = np.clip((en-st)/SAMPLE_RATE-offset, 0, 2.2)/2.2
        env = np.minimum(att,release)
        root_hz = hz(base, root-12)
        trem = 0.84 + .16*np.sin(2*np.pi*(.075+i*.006)*t)
        for j,(step,mul) in enumerate([(0,.30),(7,.12),(12,.08)]):
            f = root_hz*2**(step/12)
            pad[st:en] += instrument(f, t, 'air') * env * trem * mul
        # Root bass with an intentionally subdued second harmonic.
        audio[st:en] += instrument(root_hz/2, t, 'soft') * env * .078
    audio += pad
    # Delicate plucked pattern with measured human-like gaps and changing octave.
    beat = period / 2
    step_count = int(SECONDS / beat)
    for i in range(step_count):
        time = i*beat + float(rng.uniform(-.08,.08))
        if time < 0 or time >= SECONDS-.1:
            continue
        if rng.random() < (.14 if mood in ('memory','gentle') else .06):
            continue
        degree = scale[(i*3 + i//6) % len(scale)]
        if i%9==0:
            degree = scale[0]
        note_freq = hz(base,degree + (12 if i%11==8 else 0))
        gain = .075 + .018*(i%4==0)
        if mood == 'storm':
            gain *= .72
        place_bell(audio,note_freq,time,min(4.6,beat*2.1),gain,rng)
    # Sparse high echo-motifs, a longer-repeated narrative identity.
    for i in range(11):
        t = i*(SECONDS-4)/10 + float(rng.uniform(.0,1.3))
        note = scale[(i*5+2)%len(scale)]
        place_bell(audio,hz(base,note+12),t,5.3,.055,rng,'bell')
    # Every score has an environmental character, without imported loops/samples.
    t = TIMES
    low = lp_noise(rng,N,310)
    if mood == 'storm':
        weather = lp_noise(rng,N,22)
        swell = .28+.72*np.sin(2*np.pi*.018*t + 1.4)**2
        audio += weather*swell*.18 + low*.055
        for pos in [16.2,33.3,57.4]:
            ix=int(pos*SAMPLE_RATE);end=min(N,ix+int(3.3*SAMPLE_RATE));s=np.arange(end-ix)/SAMPLE_RATE
            audio[ix:end]+=rng.normal(0,1,end-ix)*np.exp(-s*2)*.012
    elif mood == 'wide':
        audio+=low*(.026+.012*np.sin(2*np.pi*.05*t))
    elif mood == 'clear':
        audio+=low*.012
        for st in [12,28,45,61]:
            for j in range(3):place_bell(audio,hz(base,scale[(st+j)%len(scale)]+24),st+j*.37,3.1,.019,rng)
    elif mood == 'mystic':
        audio+=low*.035 + .008*np.sin(2*np.pi*hz(base,-24)*t)*np.sin(2*np.pi*.12*t)
    elif mood == 'uneasy':
        audio+=low*.029
    else:
        audio+=low*.014
    # Short pair of progressively delayed echoes, not an impulse-file convolution.
    base_audio=audio.copy()
    for delay,gain in [(.23,.11),(.51,.085),(.88,.05)]:
        lag=round(delay*SAMPLE_RATE)
        audio[lag:]+=base_audio[:-lag]*gain
    # Make the loop seam soft by blending its last 3.5 seconds into its opening.
    fade = int(3.5*SAMPLE_RATE)
    ramp=np.linspace(0,1,fade)
    audio[-fade:] = audio[-fade:]*(1-ramp) + audio[:fade]*ramp
    audio[-320:] = audio[-320:] * np.linspace(1,0,320) + audio[:320]*np.linspace(0,1,320)
    # Force endpoint continuity even when the target phrase has a different phase.
    audio[-320:] += np.linspace(0, audio[0]-audio[-1], 320)
    # Uniform gentle listening level to avoid large cue-to-cue changes.
    peak=np.max(np.abs(audio))
    rms=np.sqrt(np.mean(audio*audio))
    target_rms=.135 if mood=='storm' else .115
    audio*=min(target_rms/max(1e-5,rms),.86/max(1e-5,peak))
    wav=np.int16(np.clip(audio,-.93,.93)*32767)
    DEST.mkdir(parents=True,exist_ok=True)
    with wave.open(str(DEST/f'{name}.wav'),'wb') as writer:
        writer.setnchannels(1);writer.setsampwidth(2);writer.setframerate(SAMPLE_RATE)
        writer.writeframes(wav.tobytes())
    print(f'{name:10s} {SECONDS}s rms={np.sqrt(np.mean((wav/32767.)**2)):.3f} peak={np.max(np.abs(wav))/32767:.3f}')


def main():
    for name,args in PIECES.items():score(name,*args)

if __name__=='__main__':main()
