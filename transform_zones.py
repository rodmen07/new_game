"""
Zone coordinate transformer - token based.
Reads setup.rs as one string, finds all calls to rect/obj/vis_wall/zone/wall,
extracts the x,y args (positions 1,2 after &mut commands), applies zone deltas.
"""
import re

SRC = "d:/Projects/new_game/src/setup.rs"

ZONES = [
    ("HOME",     -680, -320, -180,  180,    0,  320),
    ("OFFICE",    320,  680, -180,  180,    0,  320),
    ("LIBRARY",   380,  620,  330,  570,  100,  250),
    ("WELLNESS", -600, -400,  460,  660, -200,  140),
    ("CAFE",      190,  410,  540,  760,  -50,  220),
    ("CLINIC",   -410, -190,  540,  760,   50,  220),
]

def get_delta(x, y):
    for name, xmin, xmax, ymin, ymax, dx, dy in ZONES:
        if xmin <= x <= xmax and ymin <= y <= ymax:
            return (dx, dy, name)
    return (0, 0, None)

def fmt_rust(v):
    """Format number to Rust float literal style."""
    if v == int(v):
        return f"{int(v)}."
    return f"{v:.1f}"

with open(SRC, 'r', encoding='utf-8') as f:
    src = f.read()

# Pattern: match a full rect/obj/vis_wall/zone/wall call
# We find: FUNCNAME( ... &mut commands, ... X, Y, ...
# and transform X,Y if they fall in a zone

CALL_RE = re.compile(
    r'\b(rect|obj|vis_wall|zone|wall)\s*\('
    r'(?:[^)]*?)'  # anything inside parens (non-greedy)
    r'\)',
    re.DOTALL
)

# Float pattern: optional minus, digits, dot, optional more digits
FLOAT_PAT = r'(-?\d+\.(?:\d+)?)'

def transform_call(m):
    call_text = m.group(0)
    func_name = m.group(1)

    # Find all float literals in order
    floats = list(re.finditer(FLOAT_PAT, call_text))

    # We need the x and y: they are the 1st and 2nd floats after "&mut commands,"
    # Find position of "&mut commands," in call_text
    cmd_pos = call_text.find('&mut commands,')
    if cmd_pos == -1:
        return call_text

    # Get floats that appear after &mut commands,
    after_cmd_floats = [f for f in floats if f.start() > cmd_pos + len('&mut commands,')]

    if len(after_cmd_floats) < 2:
        return call_text

    x_match = after_cmd_floats[0]
    y_match = after_cmd_floats[1]

    x_val = float(x_match.group(1))
    y_val = float(y_match.group(1))

    dx, dy, zone_name = get_delta(x_val, y_val)
    if dx == 0 and dy == 0:
        return call_text

    nx = x_val + dx
    ny = y_val + dy
    nx_s = fmt_rust(nx)
    ny_s = fmt_rust(ny)

    # Replace x then y in the call text (by position, back to front)
    # Replace y first (higher index) then x
    result = list(call_text)

    # Replace y
    y_start, y_end = y_match.start(), y_match.end()
    result[y_start:y_end] = list(ny_s)

    # Recalculate x position (y replacement may have shifted, but x comes before y)
    result2 = ''.join(result)
    # Re-find x in modified text (should be at same position since x is before y)
    x_start, x_end = x_match.start(), x_match.end()
    result3 = list(result2)
    result3[x_start:x_end] = list(nx_s)

    final = ''.join(result3)

    if nx != x_val or ny != y_val:
        print(f"  {func_name}: ({x_val},{y_val}) -> ({nx},{ny}) [{zone_name}]")

    return final

result = CALL_RE.sub(transform_call, src)

with open(SRC, 'w', encoding='utf-8') as f:
    f.write(result)

print(f"\nDone. {len(src)} -> {len(result)} chars")
