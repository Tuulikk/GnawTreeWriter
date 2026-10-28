#!/usr/bin/env python3
"""GTW adoption report — GTW usage vs built-in edit usage over time.

Read-only against OpenCode's session store (~/.local/share/opencode/opencode.db).
Covers BOTH storage eras:

  - "part" table era (<= 2026-10-15): tool calls as separate part rows,
    tool name in data.tool, time in data.state.time.start
  - "session_message" era (Code Mode, > 2026-10-15): tool calls inline in
    assistant message content arrays, name in item.name; MCP tools are
    reachable only via the execute tool, so GTW usage is counted as
    execute calls whose code references gnawtreewriter

Output: weekly table of edit / write / shell / gtw counts + GTW share of
code edits. Run weekly to track the adoption steering (target: GTW/edit
>= 10% within 2 weeks of 2026-10-28, >= 25% within a month).
"""

from __future__ import annotations

import json
import sqlite3
import sys
from collections import defaultdict
from pathlib import Path

DB = Path.home() / ".local/share/opencode/opencode.db"
CODE_MODE_CUTOFF = "2026-10-15"  # execute tool first appears; storage era flips
RECENT_DAYS = 42  # how many recent days to always show, baseline or not


def rows_old_era(cur: sqlite3.Cursor) -> dict[str, dict[str, int]]:
    """Tool calls from the part-table era, keyed by day."""
    query = """
        SELECT date(json_extract(data,'$.state.time.start')/1000,'unixepoch') AS day,
               json_extract(data,'$.tool') AS tool
        FROM part
        WHERE json_extract(data,'$.type')='tool'
          AND json_extract(data,'$.state.time.start') IS NOT NULL
    """
    out: dict[str, dict[str, int]] = defaultdict(lambda: defaultdict(int))
    for day, tool in cur.execute(query):
        if tool in ("edit", "write", "bash", "shell"):
            out[day][tool] += 1
        elif tool and tool.startswith("gnawtreewriter"):
            out[day]["gtw"] += 1
    return out


def rows_new_era(cur: sqlite3.Cursor) -> dict[str, dict[str, int]]:
    """Tool calls from the session_message (Code Mode) era, keyed by day."""
    query = """
        SELECT date(json_extract(m.data,'$.time.created')/1000,'unixepoch') AS day,
               json_extract(t.value,'$.name') AS name,
               substr(coalesce(json_extract(t.value,'$.state.input.code'),''),1,300000) AS code
        FROM session_message m, json_each(json_extract(m.data,'$.content')) t
        WHERE m.type='assistant'
          AND json_extract(t.value,'$.type')='tool'
          AND json_extract(m.data,'$.time.created') IS NOT NULL
    """
    out: dict[str, dict[str, int]] = defaultdict(lambda: defaultdict(int))
    for day, name, code in cur.execute(query):
        if name in ("edit", "write", "bash", "shell"):
            out[day][name] += 1
        elif name == "execute" and code and "gnawtreewriter" in code:
            out[day]["gtw"] += 1
    return out


def main() -> int:
    if not DB.exists():
        print(f"opencode.db not found at {DB}", file=sys.stderr)
        return 1
    conn = sqlite3.connect(f"file:{DB}?mode=ro", uri=True)
    cur = conn.cursor()

    daily: dict[str, dict[str, int]] = defaultdict(lambda: defaultdict(int))
    for day, counters in rows_old_era(cur).items():
        for tool, n in counters.items():
            daily[day][tool] += n
    for day, counters in rows_new_era(cur).items():
        for tool, n in counters.items():
            daily[day][tool] += n
    conn.close()

    # Weekly aggregation (ISO week over daily counts).
    weekly: dict[str, dict[str, int]] = defaultdict(lambda: defaultdict(int))
    for day, counters in daily.items():
        iso = day[:10]
        year, week, _ = __import__("datetime").date.fromisoformat(iso).isocalendar()
        week_key = f"{year}-W{week:02d}"
        for tool, n in counters.items():
            weekly[week_key][tool] += n

    weeks = sorted(weekly)
    recent = set()
    if weeks:
        recent_days = sorted(daily)[-RECENT_DAYS:]
        for day in recent_days:
            year, week, _ = __import__("datetime").date.fromisoformat(day).isocalendar()
            recent.add(f"{year}-W{week:02d}")

    print(f"{'week':<10} {'edit':>6} {'write':>6} {'shell':>7} {'gtw':>5} {'gtw/edit':>9}")
    for wk in weeks:
        c = weekly[wk]
        edit, gtw = c.get("edit", 0), c.get("gtw", 0)
        ratio = f"{(gtw / edit * 100):.1f}%" if edit else "-"
        marker = " *" if wk in recent else ""
        print(f"{wk:<10} {edit:>6} {c.get('write', 0):>6} {c.get('bash', 0) + c.get('shell', 0):>7} "
              f"{gtw:>5} {ratio:>9}{marker}")

    print("\n(*) recent weeks. GTW counted as direct gnawtreewriter_* calls before "
          f"{CODE_MODE_CUTOFF}, as execute-calls referencing gnawtreewriter after.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
