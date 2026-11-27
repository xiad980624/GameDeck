"""Generate GameDeck's original UI cues; no third-party recordings or packages."""
import math
import struct
import wave
from pathlib import Path

RATE = 44100
OUTPUT = Path(__file__).resolve().parents[1] / "apps/desktop/public/audio"


def tone(t, duration, frequency, end_frequency, gain):
    if not 0 <= t < duration:
        return 0.0
    attack = min(1.0, t / 0.003)
    release = min(1.0, (duration - t) / 0.012)
    envelope = attack * release * math.exp(-4.0 * t / duration)
    phase = 2 * math.pi * (frequency * t + (end_frequency - frequency) * t * t / (2 * duration))
    return gain * envelope * (math.sin(phase) + 0.15 * math.sin(2 * phase))


def render(name, duration, notes):
    samples = []
    for i in range(round(duration * RATE)):
        t = i / RATE
        sample = sum(tone(t - start, length, hz, end_hz, gain)
                     for start, length, hz, end_hz, gain in notes)
        assert abs(sample) < 0.3, "UI cue must stay quiet and never clip"
        samples.append(round(sample * 32767))
    samples[-1] = 0
    with wave.open(str(OUTPUT / f"{name}.wav"), "wb") as audio:
        audio.setparams((1, 2, RATE, 0, "NONE", "not compressed"))
        audio.writeframes(struct.pack(f"<{len(samples)}h", *samples))


if __name__ == "__main__":
    OUTPUT.mkdir(parents=True, exist_ok=True)
    render("select", 0.065, [(0, 0.065, 1050, 650, 0.16)])
    render("launch", 0.3, [(0, 0.15, 660, 660, 0.19),
                           (0.085, 0.215, 990, 990, 0.19)])
