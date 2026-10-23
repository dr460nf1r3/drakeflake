#!/usr/bin/env python3
"""Generate DrakeFlake's icons: a faceted ice crystal whose bottom arm is a
curled dragon tail, with a bevelled hexagon head and slit eyes.

    scripts/make-icons.py   # data/icons/*.svg, src/qml/images/, docs/theme/favicon.svg

Small sizes get their own drawing (no side branches, thicker shapes): scaling
the detailed one down turns the branches into noise below ~32 px.
"""

import math
import pathlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "data" / "icons"

DARK = dict(hi="#c6f0f8", lo="#4fa3bd", face="#181825", eye="#cba6f7")
LIGHT = dict(hi="#6f9cfa", lo="#0f45b5", face="#e6e9ef", eye="#8839ef")
TILE = "#1e1e2e"
BADGE = "#cba6f7"


LO_MARK = ' data-lo="1"'


def pts(points):
    return " ".join(f"{x:.2f},{y:.2f}" for x, y in points)


def polygon(points, fill, shade=False):
    marker = LO_MARK if shade and fill == "currentColor" else ""
    return f'<polygon points="{pts(points)}" fill="{fill}"{marker}/>'


def diamond(p, x0, y0, angle, length, half_width, widest=0.3):
    """A two-tone diamond from (x0, y0) pointing at `angle` degrees (0 = up):
    one side lit, the other shaded, like a cut crystal."""
    r = math.radians(angle)
    dx, dy = math.sin(r), -math.cos(r)
    px, py = -dy, dx
    mx, my = x0 + dx * length * widest, y0 + dy * length * widest
    tip = (x0 + dx * length, y0 + dy * length)
    left = (mx - px * half_width, my - py * half_width)
    right = (mx + px * half_width, my + py * half_width)
    return polygon([(x0, y0), left, tip], p["hi"]) + polygon([(x0, y0), right, tip], p["lo"], True)


def arm(p, small):
    if small:
        return diamond(p, 0, -30, 0, 78, 12, 0.25)
    return (
        diamond(p, 0, -34, 0, 80, 7.5, 0.22)
        + diamond(p, 0, -64, -55, 26, 4.4)
        + diamond(p, 0, -64, 55, 26, 4.4)
    )


def bezier(p0, c1, c2, p3, n=28):
    out = []
    for i in range(n + 1):
        t = i / n
        u = 1 - t
        x = u**3 * p0[0] + 3 * u * u * t * c1[0] + 3 * u * t * t * c2[0] + t**3 * p3[0]
        y = u**3 * p0[1] + 3 * u * u * t * c1[1] + 3 * u * t * t * c2[1] + t**3 * p3[1]
        dx = 3 * u * u * (c1[0] - p0[0]) + 6 * u * t * (c2[0] - c1[0]) + 3 * t * t * (p3[0] - c2[0])
        dy = 3 * u * u * (c1[1] - p0[1]) + 6 * u * t * (c2[1] - c1[1]) + 3 * t * t * (p3[1] - c2[1])
        length = math.hypot(dx, dy) or 1
        out.append(((x, y), (dx / length, dy / length)))
    return out


def tail(p, small):
    """The curled tail: tapered, lit on one side, shaded on the other."""
    path = bezier((0, 26), (0, 74), (46, 100), (64, 80))
    path += bezier((64, 80), (76, 66), (58, 54), (50, 66))[1:]
    start, end = (11, 4.5) if small else (7.5, 2.2)
    left, right, centre = [], [], []
    for i, ((x, y), (tx, ty)) in enumerate(path):
        width = start + (end - start) * i / (len(path) - 1)
        nx, ny = -ty, tx
        left.append((x + nx * width, y + ny * width))
        right.append((x - nx * width, y - ny * width))
        centre.append((x, y))
    back = list(reversed(centre))
    return polygon(left + back, p["hi"]) + polygon(right + back, p["lo"], True)


def head(p, small):
    outer_r, inner_r = (34, 20) if small else (30, 21)
    def corner(r, a):
        return (r * math.sin(math.radians(a)), -r * math.cos(math.radians(a)))

    outer = [corner(outer_r, a) for a in range(0, 360, 60)]
    inner = [corner(inner_r, a) for a in range(0, 360, 60)]
    out = ""
    for i in range(6):
        j = (i + 1) % 6
        lit = i in (0, 4, 5)
        out += polygon([outer[i], outer[j], inner[j], inner[i]], p["hi"] if lit else p["lo"], not lit)
    if p["face"] != "none":
        out += polygon(inner, p["face"])
    scale = 1.25 if small else 1.0
    for side in (-1, 1):
        eye = [(side * 16, -7), (side * 4, -1.5), (side * 13, 1.5)]
        out += polygon([(x * scale, y * scale) for x, y in eye], p["eye"])
    return out


# Plasma recolours symbolic tray icons through these classes (light and dark
# panels): shapes use currentColor, the shaded facets a lower opacity.
MONO = dict(hi="currentColor", a="currentColor", lo="currentColor", face="none", eye="currentColor")
MONO_STYLE = (
    '<style id="current-color-scheme">'
    ".ColorScheme-Text{color:#232629}.ColorScheme-Highlight{color:#3daee9}</style>"
)


def mono(body):
    """Shaded facets at 55 %, so the cut-ice look survives in one colour."""
    out = []
    for part in body.split("/>"):
        if not part:
            continue
        if 'fill="currentColor"' in part and LO_MARK in part:
            part = part.replace(LO_MARK, "") + ' fill-opacity="0.55"'
        out.append(part + "/>")
    return "".join(out)


def mark(p, small=False):
    arms = "".join(
        f'<g transform="rotate({angle})">{arm(p, small)}</g>' for angle in (0, 60, 120, 240, 300)
    )
    return arms + tail(p, small) + head(p, small)


def svg(body, title):
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="-120 -120 240 240" '
        f'width="240" height="240"><title>{title}</title>{body}</svg>\n'
    )


def tiled(body, scale):
    return (
        f'<rect x="-116" y="-116" width="232" height="232" rx="54" fill="{TILE}"/>'
        f'<g transform="translate(0 -6) scale({scale})">{body}</g>'
    )


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    files = {
        "drakeflake.svg": svg(tiled(mark(DARK), 0.76), "DrakeFlake"),
        "drakeflake-small.svg": svg(tiled(mark(DARK, small=True), 0.8), "DrakeFlake"),
        "drakeflake-logo.svg": svg(mark(DARK), "DrakeFlake"),
        "drakeflake-logo-light.svg": svg(mark(LIGHT), "DrakeFlake"),
        "drakeflake-tray.svg": svg(MONO_STYLE + f'<g class="ColorScheme-Text">{mono(mark(MONO, small=True))}</g>', "DrakeFlake"),
        "drakeflake-tray-update.svg": svg(
            MONO_STYLE
            + f'<g class="ColorScheme-Text">{mono(mark(MONO, small=True))}</g>'
            + '<circle class="ColorScheme-Highlight" fill="currentColor" cx="78" cy="-78" r="32"/>',
            "DrakeFlake: updates available",
        ),
    }
    for name, text in files.items():
        (OUT / name).write_text(text)
    images = ROOT / "src" / "qml" / "images"
    images.mkdir(parents=True, exist_ok=True)
    for name in ("drakeflake-logo.svg", "drakeflake-logo-light.svg"):
        (images / name).write_text(files[name])
    (ROOT / "docs" / "theme").mkdir(parents=True, exist_ok=True)
    (ROOT / "docs" / "theme" / "favicon.svg").write_text(files["drakeflake-small.svg"])
    print("\n".join(sorted(files)))


if __name__ == "__main__":
    main()
