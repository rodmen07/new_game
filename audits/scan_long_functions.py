"""Scan Rust source files for functions exceeding a line-count threshold."""
import sys
import re
from pathlib import Path

THRESHOLD = 100
if len(sys.argv) > 2:
    try:
        THRESHOLD = int(sys.argv[2])
    except ValueError:
        print(f"Error: threshold must be an integer, got {sys.argv[2]!r}.")
        sys.exit(1)


def scan(src_dir: str) -> list[tuple[str, str, int, int]]:
    results = []
    for path in sorted(Path(src_dir).rglob("*.rs")):
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
        i = 0
        while i < len(lines):
            m = re.match(r"^(\s*)(pub\s+)?fn\s+(\w+)", lines[i])
            if m:
                indent = len(m.group(1))
                name = m.group(3)
                start = i + 1  # 1-indexed
                depth = 0
                for j in range(i, len(lines)):
                    depth += lines[j].count("{") - lines[j].count("}")
                    if depth <= 0 and j > i:
                        length = j - i + 1
                        if length > THRESHOLD:
                            rel = path.as_posix()
                            results.append((rel, name, start, length))
                        i = j + 1
                        break
                else:
                    i += 1
            else:
                i += 1
    return results


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(f"Usage: python {sys.argv[0]} <src_dir> [threshold]")
        sys.exit(1)
    hits = scan(sys.argv[1])
    if hits:
        print(f"Functions over {THRESHOLD} lines:\n")
        for path, name, line, length in hits:
            print(f"  {path}:{line}  {name}()  ({length} lines)")
        print(f"\n{len(hits)} function(s) found.")
    else:
        print(f"No functions over {THRESHOLD} lines.")
