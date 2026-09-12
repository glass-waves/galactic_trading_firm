"""step 2,3,6: taken vs not taken, missed winners, failing-condition ranking, base rates, episodes,
time-of-day. reads bars_<year>.csv, prints markdown tables to stdout (also written to analysis.md)."""
import csv, os, sys, collections, statistics as st
from common import OUT, YEARS

COND = ['c_fail', 't_s5_fail', 't_lag_fail', 't_1h_fail', 'k_s5_fail', 'k_1h_fail']
CN = {'c_fail':'composite<=-0.35', 't_s5_fail':'thrust s5m<=-0.50', 't_lag_fail':'thrust lag>=0.10', 't_1h_fail':'thrust s1h<=0.0', 'k_s5_fail':'core s5m<=-0.40', 'k_1h_fail':'core s1h<=-0.30'}
THRUST = ['c_fail','t_s5_fail','t_lag_fail','t_1h_fail']; CORE = ['c_fail','k_s5_fail','k_1h_fail']

def load(year):
    rows = []
    with open(os.path.join(OUT, f'bars_{year}.csv')) as f:
        for r in csv.DictReader(f):
            for k in ('opp_pnl_pct','opp_full_pct','ret_1155','mfe90','mae90'): r[k] = float(r[k])
            for k in ('hm','near_miss'): r[k] = int(r[k])
            for k in ('comp','s1m','s5m','s1h'): r[k] = float(r[k]) if r[k] else None
            for k in COND: r[k] = int(r[k]) if r[k] != '' else None
            rows.append(r)
    return rows

def M(g):
    v = list(g); return st.mean(v) if v else 0.0
def pct(a, b): return f"{100*a/b:.1f}%" if b else '-'
def dist(vals):
    if not vals: return dict(n=0, mean=0, med=0, hit=0)
    return dict(n=len(vals), mean=st.mean(vals), med=st.median(vals), hit=sum(1 for v in vals if v > 0)/len(vals))
def dstr(d, key='opp'):
    return f"{d['n']} | {100*d['mean']:+.3f}% | {100*d['med']:+.3f}% | {100*d['hit']:.1f}%"
def quantile(vals, q):
    s = sorted(vals); k = int(q * (len(s) - 1)); return s[k]
def episodes(rows):
    """count runs of consecutive bars (same ticker-day, hm step 1)."""
    by = collections.defaultdict(list)
    for r in rows: by[(r['date'], r['ticker'])].append(r['hm'])
    n = 0
    for k, hms in by.items():
        hms.sort(); prev = None
        for h in hms:
            if prev is None or h != prev + 1: n += 1
            prev = h
    return n, len(by)

out = open(os.path.join(OUT, 'analysis.md'), 'w')
def P(*a):
    s = ' '.join(str(x) for x in a); print(s); out.write(s + '\n')

all_rows = {}
for y in YEARS: all_rows[y] = load(y)

# ---------- sanity: near_miss vs my flags ----------
P("## sanity checks\n")
for y in YEARS:
    rows = all_rows[y]
    nm = [r for r in rows if r['near_miss'] == 1]
    bad = sum(1 for r in nm if r['c_fail'] != 0)
    free_pass = sum(1 for r in rows if r['cat'] == 'FREE' and r['win'])
    taken_nopass = sum(1 for r in rows if r['cat'] == 'TAKEN' and not r['win'])
    P(f"- {y}: near_miss bars={len(nm)}, of which composite condition failed by my eval={bad} (expect 0); FREE bars passing a window={free_pass} (expect 0); TAKEN bars failing both windows={taken_nopass} (expect 0)")

# ---------- A. categories ----------
P("\n## A. all 09:30-11:29 bars by engine category (count | mean opp_pnl | median | hit-rate>0)\n")
P("| year | category | n | mean opp | median | hit | mean full-stack | mean ret_1155 | mean mfe90 |")
P("|---|---|---|---|---|---|---|---|---|")
for y in YEARS:
    rows = all_rows[y]
    cats = collections.defaultdict(list)
    for r in rows:
        key = r['cat'] if r['cat'] != 'GATED' else 'GATED:' + r['gate']
        cats[key].append(r)
    for k in sorted(cats):
        v = cats[k]; d = dist([r['opp_pnl_pct'] for r in v])
        P(f"| {y} | {k} | {d['n']} | {100*d['mean']:+.3f}% | {100*d['med']:+.3f}% | {100*d['hit']:.1f}% | {100*M(r['opp_full_pct'] for r in v):+.3f}% | {100*M(r['ret_1155'] for r in v):+.3f}% | {100*M(r['mfe90'] for r in v):.3f}% |")

# ---------- C. missed winners ----------
P("\n## C. 'very successful' bars: top-5% opp_pnl threshold per year, and >= +1.0%\n")
P("| year | top5% threshold | n top5 | TAKEN | IN_POSITION | GATED | FREE | n >=1% | TAKEN | IN_POSITION | GATED | FREE |")
P("|---|---|---|---|---|---|---|---|---|---|---|---|")
thr = {}
winners = {}
for y in YEARS:
    rows = [r for r in all_rows[y] if r['cat'] != 'NOSCORE']
    t = quantile([r['opp_pnl_pct'] for r in rows], 0.95); thr[y] = t
    top = [r for r in rows if r['opp_pnl_pct'] >= t]; big = [r for r in rows if r['opp_pnl_pct'] >= 0.01]
    winners[y] = (top, big)
    def cc(v): 
        c = collections.Counter(r['cat'] for r in v); return c
    a = cc(top); b = cc(big)
    P(f"| {y} | {100*t:+.3f}% | {len(top)} | {a['TAKEN']} ({pct(a['TAKEN'],len(top))}) | {a['IN_POSITION']} ({pct(a['IN_POSITION'],len(top))}) | {a['GATED']} ({pct(a['GATED'],len(top))}) | {a['FREE']} ({pct(a['FREE'],len(top))}) | {len(big)} | {b['TAKEN']} ({pct(b['TAKEN'],len(big))}) | {b['IN_POSITION']} ({pct(b['IN_POSITION'],len(big))}) | {b['GATED']} ({pct(b['GATED'],len(big))}) | {b['FREE']} ({pct(b['FREE'],len(big))}) |")
P("\nepisodes (runs of consecutive qualifying bars per ticker-day) and distinct ticker-days for the FREE missed winners:\n")
P("| year | def | FREE bars | episodes | ticker-days | GATED bars | episodes | IN_POSITION bars | episodes |")
P("|---|---|---|---|---|---|---|---|---|")
for y in YEARS:
    for name, v in zip(('top5', '>=1%'), winners[y]):
        fr = [r for r in v if r['cat'] == 'FREE']; g = [r for r in v if r['cat'] == 'GATED']; ip = [r for r in v if r['cat'] == 'IN_POSITION']
        e1, d1 = episodes(fr); e2, _ = episodes(g); e3, _ = episodes(ip)
        P(f"| {y} | {name} | {len(fr)} | {e1} | {d1} | {len(g)} | {e2} | {len(ip)} | {e3} |")

# ---------- D. failing conditions among FREE missed winners ----------
P("\n## D. which conditions fail on the FREE missed winners (bars and episodes)\n")
P("for each window: 'only' = that condition is the ONLY failing condition of the window (a single loosening would admit the bar); 'among' = it fails (possibly with others).\n")
for name_i, name in enumerate(('top5', '>=1%')):
    P(f"\n### definition: {name}\n")
    P("| year | window | n FREE winners | " + " | ".join(f"{CN[c]} only / among" for c in THRUST) + " | none-fail (other window took it) |")
    P("|---|---|---|" + "---|" * (len(THRUST) + 1))
    for y in YEARS:
        fr = [r for r in winners[y][name_i] if r['cat'] == 'FREE']
        for wname, conds in (('thrust', THRUST), ('core', CORE)):
            only = collections.Counter(); among = collections.Counter(); only_ep = collections.defaultdict(list); none = 0
            for r in fr:
                fails = [c for c in conds if r[c]]
                if not fails: none += 1
                for c in fails: among[c] += 1
                if len(fails) == 1: only[fails[0]] += 1; only_ep[fails[0]].append(r)
            cells = []
            for c in conds:
                ep = episodes(only_ep[c])[0] if only_ep[c] else 0
                cells.append(f"{only[c]} ({ep} ep) / {among[c]}")
            if wname == 'core': cells += ['-'] * (len(THRUST) - len(CORE))
            P(f"| {y} | {wname} | {len(fr)} | " + " | ".join(cells) + f" | {none} |")

# single-fix admission: which ONE loosening (either window) would admit the most FREE winners, pooled and per year
P("\n### single-condition admission: FREE missed winners that exactly ONE condition (on either window) keeps out\n")
P("| year | def | FREE winners | admitted by relaxing composite | thrust s5m | thrust lag | thrust s1h | core s5m | core s1h | need >=2 changes |")
P("|---|---|---|---|---|---|---|---|---|---|")
for y in YEARS:
    for name_i, name in enumerate(('top5', '>=1%')):
        fr = [r for r in winners[y][name_i] if r['cat'] == 'FREE']
        adm = collections.Counter(); multi = 0
        for r in fr:
            tf = [c for c in THRUST if r[c]]; kf = [c for c in CORE if r[c]]
            single = set()
            if len(tf) == 1: single.add(tf[0])
            if len(kf) == 1: single.add(kf[0])
            if not single: multi += 1
            for c in single: adm[c] += 1
        P(f"| {y} | {name} | {len(fr)} | {adm['c_fail']} | {adm['t_s5_fail']} | {adm['t_lag_fail']} | {adm['t_1h_fail']} | {adm['k_s5_fail']} | {adm['k_1h_fail']} | {multi} |")

# ---------- E. base rates: the exclusion set of each condition ----------
P("\n## E. base rate: what each condition EXCLUDES (FREE bars where every other condition of that window passes and only this one fails)\n")
P("columns: bars | episodes | ticker-days | mean opp_pnl | median | hit-rate | share top5 | share >=1% | mean full-stack pnl | mean ret_1155 | for reference the TAKEN bars of that year\n")
P("| year | condition | bars | episodes | days | mean opp | median | hit | top5 share | >=1% share | mean full | mean ret1155 |")
P("|---|---|---|---|---|---|---|---|---|---|---|---|")
excl_rows = collections.defaultdict(list)
for y in YEARS:
    rows = all_rows[y]
    t5 = thr[y]
    def line(label, v):
        if not v:
            P(f"| {y} | {label} | 0 | | | | | | | | | |"); return
        d = dist([r['opp_pnl_pct'] for r in v]); e, dd = episodes(v)
        P(f"| {y} | {label} | {d['n']} | {e} | {dd} | {100*d['mean']:+.3f}% | {100*d['med']:+.3f}% | {100*d['hit']:.1f}% | {pct(sum(1 for r in v if r['opp_pnl_pct']>=t5), len(v))} | {pct(sum(1 for r in v if r['opp_pnl_pct']>=0.01), len(v))} | {100*M(r['opp_full_pct'] for r in v):+.3f}% | {100*M(r['ret_1155'] for r in v):+.3f}% |")
    line('TAKEN (reference)', [r for r in rows if r['cat'] == 'TAKEN'])
    line('ALL FREE (reference)', [r for r in rows if r['cat'] == 'FREE'])
    for wname, conds in (('thrust', THRUST), ('core', CORE)):
        for c in conds:
            v = [r for r in rows if r['cat'] == 'FREE' and r[c] and not any(r[o] for o in conds if o != c)]
            excl_rows[(wname, c)].extend(v)
            line(f"{wname}: only {CN[c]} fails", v)
    # graded distance buckets for the headline conditions
    for label, cond, key, buckets in (
        ('thrust: only composite fails, comp in', 'c_fail', 'comp', [(-0.35,-0.30),(-0.30,-0.25),(-0.25,0.0)]),
        ('core: only composite fails, comp in', 'c_fail', 'comp', [(-0.35,-0.30),(-0.30,-0.25),(-0.25,0.0)]),
        ('thrust: only s5m fails, s5m in', 't_s5_fail', 's5m', [(-0.50,-0.40),(-0.40,-0.30),(-0.30,0.0)]),
        ('core: only s5m fails, s5m in', 'k_s5_fail', 's5m', [(-0.40,-0.30),(-0.30,-0.20),(-0.20,0.0)]),
        ('thrust: only s1h fails, s1h in', 't_1h_fail', 's1h', [(0.0,0.10),(0.10,0.20),(0.20,1.0)]),
        ('core: only s1h fails, s1h in', 'k_1h_fail', 's1h', [(-0.30,-0.15),(-0.15,0.0),(0.0,1.0)]),
    ):
        conds = THRUST if label.startswith('thrust') else CORE
        for lo, hi in buckets:
            v = [r for r in rows if r['cat'] == 'FREE' and r[cond] and not any(r[o] for o in conds if o != cond) and lo < r[key] <= hi]
            line(f"{label} ({lo:+.2f},{hi:+.2f}]", v)
    # lag distance
    for lo, hi in ((0.05, 0.10), (0.0, 0.05), (-9, 0.0)):
        v = [r for r in rows if r['cat'] == 'FREE' and r['t_lag_fail'] and not any(r[o] for o in THRUST if o != 't_lag_fail') and lo < min(r['s1m'] - r['s5m'], r['s1h'] - r['s5m']) <= hi]
        line(f"thrust: only lag fails, lag in ({lo:+.2f},{hi:+.2f}]", v)

P("\n### pooled 2022-2026 exclusion sets\n")
P("| condition | bars | episodes | days | mean opp | median | hit | mean full | mean ret1155 |")
P("|---|---|---|---|---|---|---|---|---|")
for (wname, c), v in excl_rows.items():
    d = dist([r['opp_pnl_pct'] for r in v]); e, dd = episodes(v)
    P(f"| {wname}: only {CN[c]} fails | {d['n']} | {e} | {dd} | {100*d['mean']:+.3f}% | {100*d['med']:+.3f}% | {100*d['hit']:.1f}% | {100*M(r['opp_full_pct'] for r in v):+.3f}% | {100*M(r['ret_1155'] for r in v):+.3f}% |")

# ---------- F. time of day and ticker ----------
P("\n## F. where the FREE missed winners (top5 def) sit in time and ticker, vs all FREE bars\n")
P("| year | bucket | all FREE bars | FREE top5 winners | winner rate | FREE >=1% | rate |")
P("|---|---|---|---|---|---|---|")
for y in YEARS:
    rows = [r for r in all_rows[y] if r['cat'] == 'FREE']; t5 = thr[y]
    def bucket(hm):
        if hm < 580: return '09:30-09:39'
        if hm < 600: return '09:40-09:59'
        if hm < 630: return '10:00-10:29'
        if hm < 660: return '10:30-10:59'
        return '11:00-11:29'
    for b in ('09:30-09:39','09:40-09:59','10:00-10:29','10:30-10:59','11:00-11:29'):
        v = [r for r in rows if bucket(r['hm']) == b]; w = [r for r in v if r['opp_pnl_pct'] >= t5]; w1 = [r for r in v if r['opp_pnl_pct'] >= 0.01]
        P(f"| {y} | {b} | {len(v)} | {len(w)} | {pct(len(w),len(v))} | {len(w1)} | {pct(len(w1),len(v))} |")
    for tk in ('AMZN','AAPL','NVDA','MSFT'):
        v = [r for r in rows if r['ticker'] == tk]; w = [r for r in v if r['opp_pnl_pct'] >= t5]; w1 = [r for r in v if r['opp_pnl_pct'] >= 0.01]
        P(f"| {y} | {tk} | {len(v)} | {len(w)} | {pct(len(w),len(v))} | {len(w1)} | {pct(len(w1),len(v))} |")
P("\nfor comparison, the TAKEN bars by time bucket and ticker (count, mean opp_pnl):\n")
P("| year | 09:30-09:39 | 09:40-09:59 | 10:00-10:29 | 10:30-10:59 | 11:00-11:29 | AMZN | AAPL | NVDA | MSFT |")
P("|---|---|---|---|---|---|---|---|---|---|")
for y in YEARS:
    rows = [r for r in all_rows[y] if r['cat'] == 'TAKEN']
    cells = []
    for b in ('09:30-09:39','09:40-09:59','10:00-10:29','10:30-10:59','11:00-11:29'):
        v = [r for r in rows if bucket(r['hm']) == b]; cells.append(f"{len(v)} ({100*M(r['opp_pnl_pct'] for r in v):+.2f}%)" if v else '0')
    for tk in ('AMZN','AAPL','NVDA','MSFT'):
        v = [r for r in rows if r['ticker'] == tk]; cells.append(f"{len(v)} ({100*M(r['opp_pnl_pct'] for r in v):+.2f}%)" if v else '0')
    P(f"| {y} | " + " | ".join(cells) + " |")
# first-10-minute detail for the exclusion sets (scores not settled?)
P("\nexclusion-set bars in the first 10 minutes vs later (mean opp_pnl, hit):\n")
P("| condition | first10 bars | mean | hit | later bars | mean | hit |")
P("|---|---|---|---|---|---|---|")
for (wname, c), v in excl_rows.items():
    a = [r['opp_pnl_pct'] for r in v if r['hm'] < 580]; b = [r['opp_pnl_pct'] for r in v if r['hm'] >= 580]
    da = dist(a); db = dist(b)
    P(f"| {wname}: only {CN[c]} fails | {da['n']} | {100*da['mean']:+.3f}% | {100*da['hit']:.1f}% | {db['n']} | {100*db['mean']:+.3f}% | {100*db['hit']:.1f}% |")
out.close()
