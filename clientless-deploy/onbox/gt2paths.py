import json, os
AN = "/home/sdanced/clientless/analysis"
gt = json.load(open(os.path.join(AN, "goal_tree.json")))
goal = gt.get("goal", {}); subs = gt.get("subgoals", [])
done = sum(1 for s in subs if str(s.get("status", "")).lower() in ("done", "complete", "verified"))
paths = []
for s in subs:
    nm = (str(s.get("id", "?")) + " " + str(s.get("title", ""))).strip()
    hyp = (s.get("next_substep") or s.get("done_when") or "")[:160]
    paths.append({"name": nm, "status": s.get("status", "?"), "hypothesis": hyp})
out = {"goals": {goal.get("key", "clientless_albion_bot"): {
    "metric_name": goal.get("metric", "subgoals_done /4"),
    "current": done, "target": len(subs) or 4, "tick": goal.get("tick"),
    "paths": paths}},
    "_note": "generated from goal_tree.json each tick (box uses flat files, not SpacetimeDB paths.json)"}
json.dump(out, open(os.path.join(AN, "paths.json"), "w"), indent=2)
print("wrote paths.json:", len(paths), "paths, current", done, "/", len(subs))
