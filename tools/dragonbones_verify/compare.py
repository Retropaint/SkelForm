"""Compare DragonBonesCPP runtime poses (harness out.json) with SkelForm's own poses (<name>_ref.json).

usage: python compare.py <name>_ref.json <harness_out.json>
"""
import json, math, sys

ref = json.load(open(sys.argv[1]))
db = json.load(open(sys.argv[2]))
names = ref["boneNames"]

stats = {}


def note(kind, err, where):
    s = stats.setdefault(kind, {"max": 0.0, "where": None, "n": 0, "sum": 0.0})
    s["n"] += 1
    s["sum"] += err
    if err > s["max"]:
        s["max"], s["where"] = err, where


def match_points(a, b):
    """Max distance after greedy nearest matching (vertex order may differ for image quads)."""
    pa = [(a[i], a[i + 1]) for i in range(0, len(a), 2)]
    pb = [(b[i], b[i + 1]) for i in range(0, len(b), 2)]
    worst = 0.0
    used = set()
    for p in pa:
        best, bi = 1e18, None
        for j, q in enumerate(pb):
            if j in used:
                continue
            d = math.hypot(p[0] - q[0], p[1] - q[1])
            if d < best:
                best, bi = d, j
        used.add(bi)
        worst = max(worst, best)
    return worst


def compare(rpose, dpose, where):
    for i, m in enumerate(rpose["bones"]):
        d = dpose["bones"].get(names[i])
        if d is None:
            note("missing bone", 1, f"{where} {names[i]}")
            continue
        note("bone pos", math.hypot(m[4] - d[4], m[5] - d[5]), f"{where} {names[i]}")
        note("bone linear", max(abs(m[k] - d[k]) for k in range(4)), f"{where} {names[i]}")

    # draw order: compare the relative order of slots present in both
    present = [i for i in range(len(names)) if names[i] in dpose["slots"]]
    ref_order = sorted(present, key=lambda i: rpose["slots"][i]["zRank"])
    db_order = sorted(present, key=lambda i: dpose["slots"][names[i]]["drawOrder"])
    note("draw order mismatch", 0 if ref_order == db_order else 1, where)

    for i in present:
        rs, ds = rpose["slots"][i], dpose["slots"][names[i]]
        rv, dv = rs["verts"], ds["verts"]
        if bool(rv) != bool(dv):
            note("visibility mismatch", 1, f"{where} {names[i]} ref={len(rv)} db={len(dv)}")
            continue
        note("visibility mismatch", 0, where)
        if rv:
            if len(rv) != len(dv):
                note("vertex count mismatch", 1, f"{where} {names[i]} {len(rv)} vs {len(dv)}")
            else:
                note("verts", match_points(rv, dv), f"{where} {names[i]}")
        note("color", max(abs(a - b) for a, b in zip(rs["color"], ds["color"])), f"{where} {names[i]}")


compare(ref["setup"], db["setup"], "setup")
for anim, frames in ref["animations"].items():
    dframes = db["animations"][anim]["frames"]
    for fr in frames:
        f = int(fr["dbFrame"])
        if f < len(dframes):
            compare(fr["pose"], dframes[f], f"{anim}@{f}")

for kind, s in stats.items():
    print(f"{kind:22s} max={s['max']:.4f} mean={s['sum']/max(s['n'],1):.4f} n={s['n']}  worst: {s['where']}")
