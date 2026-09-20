from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[2]
FRONTEND_API = ROOT / "frontend" / "src" / "api.ts"
BACKEND_SERVER = ROOT / "src" / "mydns" / "web" / "server.rs"

frontend = FRONTEND_API.read_text(encoding="utf-8")
backend = BACKEND_SERVER.read_text(encoding="utf-8")

frontend_paths = sorted(set(re.findall(r'''[`\"'](/api/v1/[^`\"']+)[`\"']''', frontend)))
backend_routes = sorted(set(re.findall(r'''\.route\("([^"]+)"\s*,\s*''', backend)))

missing = []
for path in frontend_paths:
    prefix = path.split("${", 1)[0]
    if prefix.endswith("/"):
        matched = any(("/api/v1" + route).startswith(prefix) for route in backend_routes)
    else:
        matched = any(("/api/v1" + route) == prefix for route in backend_routes)
    if not matched:
        missing.append(path)

if missing:
    print("Frontend/backend API contract mismatch:")
    for path in missing:
        print(f"  missing backend route coverage for {path}")
    sys.exit(1)

print(f"API contract check passed: {len(frontend_paths)} frontend paths have backend route coverage.")
