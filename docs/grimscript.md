# GrimScript Language Guide

GrimScript is a Python-like scripting language for controlling entities in VOID//SCRIPT. Scripts are deterministic — given the same world state, they always produce the same result.

## Quick Start

```python
# A simple soul script
def soul():
    if self.health < 50:
        defend()
    else:
        attack()
```

Scripts use **indentation** for blocks (like Python), **`#`** for comments, and run in a tick-based simulation where each action takes one tick.

---

## Types

| Type | Example | Notes |
|------|---------|-------|
| `int` | `42`, `-7`, `0` | 64-bit signed integer. No floats in sim. |
| `bool` | `True`, `False` | |
| `str` | `"hello"`, `'world'` | Immutable strings. |
| `None` | `None` | Null value. |
| `list` | `[1, 2, 3]` | Mutable, ordered collection. |
| `dict` | `{"a": 1, "b": 2}` | String keys, ordered by insertion. |
| `entity` | `self` | Reference to a game entity. |

### Truthiness

| Falsy | Truthy |
|-------|--------|
| `False`, `0`, `""`, `None`, `[]`, `{}` | Everything else |

---

## Variables

```python
x = 42
name = "skeleton"
lst = [1, 2, 3]
```

First assignment creates the variable. No declaration keyword needed.

### Augmented Assignment

```python
x += 5      # x = x + 5
x -= 3      # x = x - 3
x *= 2      # x = x * 2
x /= 4      # x = x / 4
```

Works on list/dict elements too: `lst[0] += 1`

---

## Operators

### Arithmetic

| Op | Description | Example |
|----|-------------|---------|
| `+` | Add / concatenate | `3 + 4` → `7`, `"a" + "b"` → `"ab"` |
| `-` | Subtract | `10 - 3` → `7` |
| `*` | Multiply | `4 * 5` → `20` |
| `/` or `//` | Floor division | `7 // 2` → `3`, `-7 // 2` → `-4` |
| `%` | Modulo (floor) | `7 % 3` → `1`, `-7 % 2` → `1` |
| `-x` | Negate | `-5` |

Division and modulo use **Python-style floor semantics** (round toward negative infinity).

### Comparison

| Op | Description |
|----|-------------|
| `==` | Equal |
| `!=` | Not equal |
| `<` | Less than |
| `>` | Greater than |
| `<=` | Less or equal |
| `>=` | Greater or equal |
| `is None` | Is None |
| `is not None` | Is not None |
| `in` | Membership (list, dict keys, substring) |
| `not in` | Not a member |

### Boolean

| Op | Description |
|----|-------------|
| `and` | Short-circuit AND |
| `or` | Short-circuit OR |
| `not` | Negation |

`and` / `or` return the deciding value, not necessarily a bool:

```python
x = 0 or 5       # x = 5
y = 3 and 0      # y = 0
```

### Precedence (lowest to highest)

1. `or`
2. `and`
3. `not`
4. `==`, `!=`, `<`, `>`, `<=`, `>=`, `is`, `in`, `not in`
5. `+`, `-`
6. `*`, `/`, `//`, `%`
7. `-x` (unary negate)
8. `()` call, `[]` index, `.` attribute

---

## Control Flow

### if / elif / else

```python
if health < 20:
    retreat()
elif health < 50:
    defend()
else:
    attack()
```

### while

```python
count = 0
while count < 5:
    print(count)
    count += 1
```

### for

```python
for item in [1, 2, 3]:
    print(item)

for i in range(5):
    print(i)         # 0, 1, 2, 3, 4

for key in my_dict:
    print(key)       # iterates over keys
```

### break / continue

```python
for i in range(10):
    if i == 5:
        break        # exit loop
    if i % 2 == 0:
        continue     # skip to next iteration
    print(i)         # prints 1, 3
```

### pass

```python
if condition:
    pass             # placeholder, does nothing
```

---

## Functions

```python
def greet(name):
    print("Hello, " + name)

def add(a, b):
    return a + b

result = add(3, 4)  # 7
greet("skeleton")   # Hello, skeleton
```

- Functions are defined with `def` and can appear anywhere in the file (collected before execution).
- `return` exits the function. Without a value, returns `None`.
- A function named `main()` is auto-called after top-level code runs.

### Recursion

```python
def factorial(n):
    if n <= 1:
        return 1
    return n * factorial(n - 1)
```

---

## Enums

Named integer constants with auto-incrementing values:

```python
enum State:
    IDLE          # 0
    WALKING       # 1
    ATTACKING     # 2

enum Priority:
    LOW = 10
    MEDIUM        # 11
    HIGH          # 12

current = State.IDLE
print(current)       # 0
```

Use with `match` for clean dispatch.

---

## Match / Case

Pattern matching on values:

```python
match state:
    case State.IDLE:
        print("doing nothing")
    case State.WALKING:
        print("on the move")
    case _:
        print("something else")
```

### Patterns

| Pattern | Example | Matches |
|---------|---------|---------|
| Literal | `case 42:`, `case "hello":` | Exact value |
| Negative literal | `case -1:` | Negative integers |
| Enum member | `case State.IDLE:` | Enum integer value |
| Wildcard | `case _:` | Anything (catch-all) |
| OR | `case 1 \| 2 \| 3:` | Any of the patterns |

```python
match direction:
    case 1 | 2:
        print("forward-ish")
    case -1 | -2:
        print("backward-ish")
    case 0:
        print("standing still")
```

First matching case wins. No fall-through.

---

## Lists

```python
lst = [1, 2, 3]
lst[0]               # 1
lst[-1]              # 3 (last element)
lst[0] = 99          # [99, 2, 3]
lst.append(4)        # [99, 2, 3, 4]
len(lst)             # 4
```

### List Comprehensions

```python
squares = [x * x for x in range(5)]
# [0, 1, 4, 9, 16]

evens = [x for x in range(10) if x % 2 == 0]
# [0, 2, 4, 6, 8]
```

---

## Dictionaries

```python
stats = {"health": 100, "armor": 5}
stats["health"]          # 100
stats["speed"] = 3       # add new key
stats.get("armor", 0)    # 5 (safe access with default)
stats.keys()             # ["health", "armor", "speed"]
stats.values()           # [100, 5, 3]
stats.items()            # [["health", 100], ["armor", 5], ["speed", 3]]
len(stats)               # 3

for key in stats:
    print(key, stats[key])
```

---

## Strings

```python
msg = "hello"
msg[0]               # "h"
msg[-1]              # "o"
len(msg)             # 5
"ell" in msg         # True

# Concatenation
greeting = "hello" + " " + "world"

# Escape sequences
newline = "line1\nline2"
tab = "col1\tcol2"
```

### Escape Sequences

| Sequence | Meaning |
|----------|---------|
| `\n` | Newline |
| `\t` | Tab |
| `\\` | Backslash |
| `\'` | Single quote |
| `\"` | Double quote |

---

## Entity Access

The `self` variable refers to the entity running the script.

```python
# Built-in attributes
self.pos             # position (also: self.position, self.x)
self.name            # entity name
self.type            # entity type string
self.types           # list of type tags
self.owner           # owner entity (or None)
self.alive           # True/False
self.target          # target entity (or None)

# Stats (any stat defined by mods, returns 0 if unset)
self.health
self.armor
self.speed
```

---

## Builtin Functions

### Output

| Function | Description |
|----------|-------------|
| `print(args...)` | Print values separated by spaces. Instant (no tick consumed). |

### Control

| Function | Description |
|----------|-------------|
| `wait()` | Do nothing for one tick. Consumes the tick. |

```python
print("health:", self.health)
print(1, 2, 3)    # "1 2 3"
```

### Math

| Function | Description |
|----------|-------------|
| `abs(x)` | Absolute value. |
| `min(a, b)` | Minimum of two values. |
| `max(a, b)` | Maximum of two values. |
| `percent(value, pct)` | `value * pct / 100` with banker's rounding. |
| `scale(value, num, den)` | `value * num / den` with banker's rounding. |
| `random(max)` | Random int in `[0, max)`. |
| `random(min, max)` | Random int in `[min, max)`. |

```python
percent(200, 25)     # 50 (25% of 200)
scale(100, 3, 4)     # 75 (100 * 3 / 4)
random(6)            # 0, 1, 2, 3, 4, or 5
random(1, 7)         # 1, 2, 3, 4, 5, or 6
```

`random()` is deterministic — same world seed, same tick, same entity, same call order always produces the same result.

### Collections

| Function | Description |
|----------|-------------|
| `len(x)` | Length of list, string, or dict. |
| `range(stop)` | List `[0, 1, ..., stop-1]`. |
| `range(start, stop)` | List `[start, start+1, ..., stop-1]`. |
| `range(start, stop, step)` | List with custom step. |

### Type Conversion

| Function | Description |
|----------|-------------|
| `int(x)` | Convert to int (from string, bool). |
| `str(x)` | Convert to string. |
| `type(x)` | Type name as string (`"int"`, `"str"`, `"list"`, etc.). |

---

## Methods

### List Methods

| Method | Description |
|--------|-------------|
| `lst.append(x)` | Add element to end. |

### Dict Methods

| Method | Description |
|--------|-------------|
| `d.keys()` | List of keys. |
| `d.values()` | List of values. |
| `d.items()` | List of `[key, value]` pairs. |
| `d.get(key)` | Value or `None` if missing. |
| `d.get(key, default)` | Value or `default` if missing. |

---

## Soul Scripts

Entities with a soul type run a `.gs` script each tick. There are two patterns:

### With `soul()` function (recommended)

Top-level code runs once (initialization). The `soul()` function auto-loops each tick:

```python
# Top-level: runs once
target_pos = 100

# Runs every tick
def soul():
    if self.pos < target_pos:
        walk_right()
    else:
        walk_left()
```

Global variables persist across ticks. The soul function's local variables reset each loop.

### Without `soul()`

The entire script runs once and halts. Useful for one-shot setup scripts.

---

## Commands

Commands are game actions defined by mods. Each command **consumes one tick** — the entity can only do one action per tick.

```python
def soul():
    attack()         # takes one tick
    walk_right()     # takes one tick (next tick)
    defend()         # takes one tick (tick after that)
```

Available commands depend on the entity's type. Check the mod documentation for which commands exist.

---

## Execution Model

- The simulation runs at **30 ticks per second**.
- Each tick, every entity executes its soul script until it performs an action (which consumes the tick) or halts.
- `print()` is instant — it does not consume a tick. You can call it freely for debugging.
- If a script errors, it automatically recovers next tick (resets and restarts).
- Scripts have a **10,000 instruction step limit** per tick to prevent infinite loops.

---

## Complete Example

```python
enum Mode:
    PATROL
    CHASE
    FLEE

mode = Mode.PATROL
patrol_min = 50
patrol_max = 200

def soul():
    if self.health < 20:
        mode = Mode.FLEE

    match mode:
        case Mode.PATROL:
            patrol()
        case Mode.CHASE:
            chase()
        case Mode.FLEE:
            flee()

def patrol():
    if self.pos <= patrol_min:
        walk_right()
    elif self.pos >= patrol_max:
        walk_left()
    else:
        # Pick a random direction
        if random(2) == 0:
            walk_left()
        else:
            walk_right()

def chase():
    target = self.target
    if target is None:
        return
    if target.pos < self.pos:
        walk_left()
    else:
        walk_right()

def flee():
    walk_left()
```
