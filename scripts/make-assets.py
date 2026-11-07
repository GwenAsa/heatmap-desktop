#!/usr/bin/env python3
"""Generates the README artwork in assets/. Pure standard library; re-run after changing the palette."""
import datetime, math, os, random

OUT = os.path.join(os.path.dirname(__file__), "..", "assets")
os.makedirs(OUT, exist_ok=True)
EMPTY, MUTED, TEXT = "#161b22", "#8b949e", "#e6edf3"
LEVELS = ["#0e4429", "#006d32", "#26a641", "#39d353"]       # intensity 1..4
FG = ["#ffffff", "#ffffff", "#000000", "#000000"]            # readable label colour for each level
FONT = "Inter,'Segoe UI',Ubuntu,Cantarell,system-ui,sans-serif"

def write(name, body):
    with open(os.path.join(OUT, name), "w", encoding="utf-8") as f:
        f.write(body)

def month_cells(year, month, week_start=0):
    first = datetime.date(year, month, 1)
    lead = (first.weekday() + 1 - week_start) % 7         # python: Monday=0 -> Sunday-first column
    nxt = datetime.date(year + (month == 12), month % 12 + 1, 1)
    return lead, (nxt - first).days

# ---------------------------------------------------------------- banner
def banner(h=380, dy=0, title=84):
    lead, n = month_cells(2026, 10)
    levels = [0, 2, 3, 1, 0, 3, 4, 4, 0, 1, 2, 4, 3, 0, 2, 3, 4, 4, 1, 0, 3, 4, 4, 2, 0, 1, 3, 4, 3, 2, 4]
    cs, gap, x0, y0 = 50, 7, 800, 60 + dy
    cells = []
    for i in range(7):
        cells.append(f'<text x="{x0 + i * (cs + gap) + cs / 2}" y="{y0 - 14}" font-size="13" fill="{MUTED}" text-anchor="middle">{"SMTWTFS"[i]}</text>')
    for d in range(1, n + 1):
        idx = lead + d - 1
        x, y = x0 + (idx % 7) * (cs + gap), y0 + (idx // 7) * (cs + gap)
        lv = levels[d - 1]
        fill = LEVELS[lv - 1] if lv else EMPTY
        fg = FG[lv - 1] if lv else MUTED
        ring = ' stroke="#ffffff" stroke-width="2.5"' if d == 21 else ""
        glow = ' filter="url(#glow)"' if lv == 4 else ""
        cells.append(
            f'<g class="c" style="animation-delay:{(d * 0.17) % 4:.2f}s">'
            f'<rect x="{x}" y="{y}" width="{cs}" height="{cs}" rx="11" fill="{fill}"{ring}{glow}/>'
            f'<text x="{x + cs / 2}" y="{y + cs / 2}" dy=".36em" text-anchor="middle" font-size="20" font-weight="700" fill="{fg}">{d}</text></g>')
    chips = ""
    for i, t in enumerate(["Tauri 2", "Rust", "SQLite", "React"]):
        w = 20 + 9 * len(t)
        x = 72 + sum(20 + 9 * len(p) + 10 for p in ["Tauri 2", "Rust", "SQLite", "React"][:i])
        chips += (f'<rect x="{x}" y="{300 + dy}" width="{w}" height="30" rx="15" fill="#ffffff" fill-opacity=".05" stroke="#ffffff" stroke-opacity=".14"/>'
                  f'<text x="{x + w / 2}" y="{315 + dy}" dy=".35em" text-anchor="middle" font-size="14" fill="#c9d4e0">{t}</text>')
    return f'''<svg xmlns="http://www.w3.org/2000/svg" width="1280" height="{h}" viewBox="0 0 1280 {h}" font-family="{FONT}" role="img" aria-label="Heatmap: local-first activity tracker with calendar heatmaps">
<defs>
  <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#0a0e14"/><stop offset="1" stop-color="#0f1b2d"/></linearGradient>
  <radialGradient id="aura" cx="0.78" cy="0.45" r="0.5"><stop offset="0" stop-color="#39d353" stop-opacity="0.28"/><stop offset="1" stop-color="#39d353" stop-opacity="0"/></radialGradient>
  <radialGradient id="aura2" cx="0.1" cy="0.1" r="0.5"><stop offset="0" stop-color="#4f8cff" stop-opacity="0.2"/><stop offset="1" stop-color="#4f8cff" stop-opacity="0"/></radialGradient>
  <linearGradient id="ttl" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#ffffff"/><stop offset="1" stop-color="#6ee7a0"/></linearGradient>
  <filter id="glow" x="-40%" y="-40%" width="180%" height="180%"><feGaussianBlur stdDeviation="4" result="b"/><feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge></filter>
  <style>.c{{animation:pop 4.4s ease-in-out infinite}}@keyframes pop{{0%,100%{{opacity:.6}}50%{{opacity:1}}}}
  @media (prefers-reduced-motion:reduce){{.c{{animation:none}}}}</style>
</defs>
<rect width="1280" height="{h}" rx="22" fill="url(#bg)"/><rect width="1280" height="{h}" rx="22" fill="url(#aura)"/><rect width="1280" height="{h}" rx="22" fill="url(#aura2)"/>
<rect x="0.5" y="0.5" width="1279" height="{h - 1}" rx="21.5" fill="none" stroke="#ffffff" stroke-opacity=".11"/>
<text x="68" y="{170 + dy}" font-size="{title}" font-weight="800" fill="url(#ttl)" letter-spacing="-2">Heatmap</text>
<text x="72" y="{222 + dy}" font-size="29" font-weight="600" fill="{TEXT}">Every day counts. Literally.</text>
<text x="72" y="{262 + dy}" font-size="17" fill="#7d8da1">Local-first tracking · calendar heatmaps · desktop widgets · quick capture</text>
{chips}
{"".join(cells)}
</svg>'''

write("banner.svg", banner())

# ---------------------------------------------------------------- year preview (real calendar maths)
def year_preview(year=2026):
    random.seed(11)
    cs, gap, left, top = 20, 3, 36, 70
    first = datetime.date(year, 1, 1)
    lead = (first.weekday() + 1) % 7
    days = 366 if year % 4 == 0 and (year % 100 or year % 400 == 0) else 365
    cols = math.ceil((lead + days) / 7)
    step = cs + gap
    W, H = left + cols * step + 24, top + 7 * step + 58
    out = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}" font-family="{FONT}" role="img" aria-label="A year of daily activity, every day numbered">',
           f'<rect width="{W}" height="{H}" rx="18" fill="#0d1117"/><rect x="0.5" y="0.5" width="{W - 1}" height="{H - 1}" rx="17.5" fill="none" stroke="#ffffff" stroke-opacity=".11"/>',
           f'<text x="{left}" y="38" font-size="20" font-weight="700" fill="{TEXT}">Coding · {year}</text>',
           f'<text x="{W - 24}" y="38" font-size="13" text-anchor="end" fill="{MUTED}">every date shows its number</text>']
    for i, name in enumerate(["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"]):
        out.append(f'<text x="{left - 8}" y="{top + i * step + cs / 2}" dy=".35em" font-size="10" text-anchor="end" fill="{MUTED}">{name}</text>')
    today = datetime.date(year, 10, 21)
    for i in range(days):
        d = first + datetime.timedelta(days=i)
        idx = lead + i
        x, y = left + (idx // 7) * step, top + (idx % 7) * step
        if d.day == 1:
            out.append(f'<text x="{x}" y="{top - 12}" font-size="12" fill="{MUTED}">{d.strftime("%b")}</text>')
        weekend = d.weekday() >= 5
        r = random.random()
        lv = 0 if d > today else (0 if r < (.42 if weekend else .16) else random.choice([1, 1, 2, 2, 3, 3, 4, 4, 4]))
        zero = d <= today and lv == 0 and random.random() < .12
        fill = LEVELS[lv - 1] if lv else EMPTY
        fg = FG[lv - 1] if lv else MUTED
        stroke = f' stroke="{MUTED}" stroke-dasharray="3 2"' if zero else ""
        ring = ' stroke="#ffffff" stroke-width="2"' if d == today else ""
        out.append(f'<rect x="{x}" y="{y}" width="{cs}" height="{cs}" rx="5" fill="{fill}"{stroke}{ring}/>'
                   f'<text x="{x + cs / 2}" y="{y + cs / 2}" dy=".36em" font-size="9.5" font-weight="600" text-anchor="middle" fill="{fg}" opacity="{0.45 if d > today else 1}">{d.day}</text>')
    ly = top + 7 * step + 22
    out.append(f'<text x="{left}" y="{ly + 12}" font-size="12" fill="{MUTED}">Less</text>')
    for i, c in enumerate(LEVELS):
        out.append(f'<rect x="{left + 38 + i * 22}" y="{ly}" width="16" height="16" rx="4" fill="{c}"/>')
    out.append(f'<text x="{left + 38 + 4 * 22 + 4}" y="{ly + 12}" font-size="12" fill="{MUTED}">More</text>')
    out.append(f'<rect x="{left + 250}" y="{ly}" width="16" height="16" rx="4" fill="{EMPTY}" stroke="{MUTED}" stroke-dasharray="3 2"/><text x="{left + 272}" y="{ly + 12}" font-size="12" fill="{MUTED}">recorded zero</text>')
    out.append(f'<rect x="{left + 380}" y="{ly}" width="16" height="16" rx="4" fill="{EMPTY}" stroke="#fff" stroke-width="2"/><text x="{left + 402}" y="{ly + 12}" font-size="12" fill="{MUTED}">today</text></svg>')
    return "\n".join(out)

write("year-preview.svg", year_preview())

# ---------------------------------------------------------------- wave divider
def divider():
    n, step = 48, 20
    out = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{n * step}" height="22" viewBox="0 0 {n * step} 22" role="presentation"><style>.w{{animation:wv 3.2s ease-in-out infinite}}@keyframes wv{{0%,100%{{opacity:.25}}50%{{opacity:1}}}}@media (prefers-reduced-motion:reduce){{.w{{animation:none}}}}</style>']
    for i in range(n):
        lv = int(2.5 + 1.5 * math.sin(i / 3.2)) % 4
        out.append(f'<rect class="w" style="animation-delay:{i * 0.07:.2f}s" x="{i * step + 2}" y="3" width="16" height="16" rx="4" fill="{LEVELS[lv]}"/>')
    out.append("</svg>")
    return "".join(out)

write("divider.svg", divider())

# ---------------------------------------------------------------- icons
def icon(name, glyph):
    write(f"icon-{name}.svg", f'''<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64" role="img" aria-label="{name}">
<defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#1c2635"/><stop offset="1" stop-color="#0f1620"/></linearGradient></defs>
<rect x="1" y="1" width="62" height="62" rx="16" fill="url(#g)" stroke="#2b3a52"/>{glyph}</svg>''')

G = "#39d353"
icon("heatmap", "".join(f'<rect x="{14 + c * 13}" y="{14 + r * 13}" width="10" height="10" rx="3" fill="{G}" opacity="{[.25, .45, .7, 1, .55, .85, .35, 1, .6][r * 3 + c]}"/>' for r in range(3) for c in range(3)))
icon("widgets", f'<rect x="12" y="15" width="28" height="30" rx="7" fill="none" stroke="#4f8cff" stroke-width="3" opacity=".8"/><circle cx="40" cy="39" r="12" fill="#0f1620" stroke="{G}" stroke-width="4"/><path d="M40 27a12 12 0 0 1 12 12" stroke="#ffffff" stroke-width="4" fill="none" stroke-linecap="round"/>')
icon("quick", f'<path d="M36 11 18 36h12l-3 17 19-26H34z" fill="{G}" stroke="#a8f5bb" stroke-width="1.5" stroke-linejoin="round"/>')
icon("git", f'<circle cx="21" cy="17" r="5" fill="none" stroke="{G}" stroke-width="3"/><circle cx="21" cy="47" r="5" fill="none" stroke="{G}" stroke-width="3"/><circle cx="43" cy="28" r="5" fill="none" stroke="#f0883e" stroke-width="3"/><path d="M21 22v20M21 36c0-7 22-2 22-8" stroke="{G}" stroke-width="3" fill="none" stroke-linecap="round"/>')
icon("insights", f'<rect x="14" y="36" width="7" height="14" rx="2" fill="{G}" opacity=".55"/><rect x="25" y="28" width="7" height="22" rx="2" fill="{G}" opacity=".8"/><rect x="36" y="20" width="7" height="30" rx="2" fill="{G}"/><path d="M13 24 26 17l11-4 14 7" stroke="#ffffff" stroke-width="3" fill="none" stroke-linecap="round" stroke-linejoin="round" opacity=".9"/>')
icon("privacy", f'<path d="M32 10 49 17v13c0 11-7 20-17 24C22 50 15 41 15 30V17z" fill="none" stroke="{G}" stroke-width="3.2" stroke-linejoin="round"/><path d="m24 31 6 6 11-12" stroke="#ffffff" stroke-width="4" fill="none" stroke-linecap="round" stroke-linejoin="round"/>')

# ---------------------------------------------------------------- social preview (1280x640)
write("social-preview.svg", banner(h=640, dy=130, title=96).replace('height="640" viewBox="0 0 1280 640"', 'height="640" viewBox="0 0 1280 640"'))
print("assets written:", sorted(os.listdir(OUT)))
