# Jotui

JSON-driven terminal UI engine built on [Ratatui](https://ratatui.rs). Write your backend in **any language** — just send JSON-RPC 2.0 over TCP, get a fully interactive TUI.

```
┌───────────┐             ┌──────────┐   Ratatui    ┌──────────┐
│ Backend   │ ◄──TCP───►  │  Jotui   │ ──────────►  │ Terminal │
│ (any      │  JSON-RPC   │  Engine  │   stdout     │          │
│  language)│  2.0        │  (Rust)  │              │          │
└───────────┘             └──────────┘              └──────────┘
```

## Quick Start

```bash
# Build
cargo build --release

# Run the demo (spawns Jotui as subprocess)
python examples/demo.py
```

Startup flow:
1. Your backend spawns Jotui as a subprocess
2. Jotui prints `{"port": N}` to stdout (one line)
3. Jotui takes over stdout for terminal rendering
4. Your backend connects to `127.0.0.1:N` via TCP and communicates over JSON-RPC 2.0

---

## Protocol — JSON-RPC 2.0

All communication uses [JSON-RPC 2.0](https://www.jsonrpc.org/specification) notifications with **Content-Length framing** (same as LSP):

```
Content-Length: 84\r\n
\r\n
{"jsonrpc":"2.0","method":"render","params":{"pages":{"main":{"layout":...}}}}
```

Each message has a `Content-Length` header, a blank line, then the JSON body (exactly that many bytes).

| Direction          | Method      | Purpose                       |
|--------------------|-------------|-------------------------------|
| Backend → Jotui    | `render`    | Full UI definition            |
| Backend → Jotui    | `patch`     | Incremental widget updates    |
| Backend → Jotui    | `navigate`  | Switch active page            |
| Jotui → Backend    | `event`     | User interaction              |

All messages are **notifications** (no `id` field).

---

## Messages

### `render` — Define the entire UI

Sent once at startup. Contains all pages, widgets, styles, and the active page.

```json
{
  "jsonrpc": "2.0",
  "method": "render",
  "params": {
    "styles": {
      "danger": { "fg": "red", "bold": true },
      "ok": { "fg": "green" }
    },
    "page_order": ["dashboard", "settings"],
    "pages": {
      "dashboard": {
        "layout": { "..." : "..." },
        "widgets": [ "..." ]
      },
      "settings": {
        "layout": { "..." : "..." },
        "widgets": [ "..." ]
      }
    },
    "active": "dashboard"
  }
}
```

**Params fields:**

| Field        | Type   | Required | Description                                          |
|--------------|--------|----------|------------------------------------------------------|
| `styles`     | object | no       | Named style dictionary for reuse across widgets      |
| `page_order` | array  | no       | Page display order (for tabs). Defaults to insertion order |
| `pages`      | object | yes      | Map of page ID → page definition                     |
| `active`     | string | no       | Initially active page. Defaults to first page        |

### `patch` — Update widgets

Sent anytime after render. Only include changed properties — everything else is preserved.

```json
{
  "jsonrpc": "2.0",
  "method": "patch",
  "params": {
    "page": "dashboard",
    "updates": [
      { "id": "cpu_gauge", "value": 87, "style": "danger" },
      { "id": "status", "text": "Warning!" }
    ]
  }
}
```

**Params fields:**

| Field     | Type   | Required | Description                          |
|-----------|--------|----------|--------------------------------------|
| `page`    | string | yes      | Target page ID (can be non-active)   |
| `updates` | array  | yes      | Array of partial widget updates      |

Each update must include `id`. All other fields are optional and merge on top of existing state.

**Limitations:** Patch cannot add/remove widgets or change a widget's `type`. Send a new `render` for structural changes.

### `navigate` — Switch page

```json
{
  "jsonrpc": "2.0",
  "method": "navigate",
  "params": {
    "page": "settings"
  }
}
```

Page state (including focus position) is preserved when navigating away and back.

### `event` — User interaction (Jotui → Backend)

Sent over the TCP connection with Content-Length framing.

```json
{
  "jsonrpc": "2.0",
  "method": "event",
  "params": {
    "page": "dashboard",
    "source": "log_list",
    "action": "select",
    "value": 2
  }
}
```

| Action   | When                            | Value type       |
|----------|---------------------------------|------------------|
| `select` | Item selected in list/table     | integer (index)  |
| `submit` | Enter pressed on widget         | string or null   |
| `key`    | Key press (has extra `key` field) | null            |

Key event example:

```json
{
  "jsonrpc": "2.0",
  "method": "event",
  "params": {
    "page": "dashboard",
    "source": null,
    "action": "key",
    "key": "f1"
  }
}
```

Events **not** sent to backend: scroll, focus/blur, resize (all handled internally).

---

## Page Definition

Each page has a `layout` tree and a `widgets` array.

```json
{
  "layout": {
    "children": [
      { "size": "3", "ref": "header" },
      {
        "dir": "h",
        "children": [
          { "size": "30%", "ref": "sidebar" },
          { "ref": "main" }
        ]
      },
      { "size": "1", "ref": "footer" }
    ]
  },
  "widgets": [
    { "id": "header", "type": "paragraph", "text": "My App" },
    { "id": "sidebar", "type": "list", "items": ["Home", "Settings"] },
    { "id": "main", "type": "paragraph", "text": "Content here" },
    { "id": "footer", "type": "paragraph", "text": "Status: OK" }
  ]
}
```

This produces:

```
┌─────────────────────────────────────┐
│ header (3 rows)                     │
├──────────┬──────────────────────────┤
│ sidebar  │ main (fills remaining)   │
│ (30%)    │                          │
├──────────┴──────────────────────────┤
│ footer (1 row)                      │
└─────────────────────────────────────┘
```

---

## Layout Nodes

A layout node is either a **container** (has `children`) or a **leaf** (has `ref` pointing to a widget ID). Never both.

| Field      | Type    | Default | Description                              |
|------------|---------|---------|------------------------------------------|
| `dir`      | string  | `"v"`   | `"v"` (vertical) or `"h"` (horizontal)  |
| `size`     | string  | `"*"`   | Size constraint (see below)              |
| `margin`   | integer | `0`     | Margin in cells around this node         |
| `children` | array   | —       | Child layout nodes (containers only)     |
| `ref`      | string  | —       | Widget ID (leaves only)                  |

### Size Constraints

| Value   | Meaning                    | Example              |
|---------|----------------------------|----------------------|
| `"*"`   | Fill remaining space       | `{ "ref": "main" }` |
| `"50%"` | Percentage of parent       | `{ "size": "50%", "ref": "sidebar" }` |
| `"3"`   | Fixed number of rows/cols  | `{ "size": "3", "ref": "header" }`    |
| `">5"`  | Minimum 5 cells            | `{ "size": ">5", "ref": "panel" }`    |
| `"<20"` | Maximum 20 cells           | `{ "size": "<20", "ref": "panel" }`   |

---

## Widgets

### Common Properties

Every widget supports these:

| Property  | Type          | Default | Description                                       |
|-----------|---------------|---------|---------------------------------------------------|
| `id`      | string        | —       | **Required.** Unique within the page              |
| `type`    | string        | —       | **Required.** Widget type name                    |
| `visible` | boolean       | `true`  | If false, space is reserved but widget is blank   |
| `border`  | string        | varies  | `"none"`, `"plain"`, `"rounded"`, `"double"`, `"thick"` |
| `title`   | string        | `""`    | Title in the border (ignored if border is `"none"`) |
| `style`   | string/object | varies  | Style object or reference to `styles` dictionary  |

### Style Object

```json
{ "fg": "cyan", "bg": "black", "bold": true, "italic": false, "underline": false }
```

**Colors:** `"red"`, `"green"`, `"blue"`, `"yellow"`, `"cyan"`, `"magenta"`, `"gray"`, `"dark_gray"`, `"white"`, `"black"`, `"reset"`, `"#FF5500"` (hex), `"color(214)"` (256-palette)

**Style references:** When `style` is a string (e.g. `"danger"`), it resolves from the `styles` dictionary in the `render` message.

### Styled Text

Text fields accept plain strings or span arrays for inline styling:

```json
"text": "Simple string"
```

```json
"text": [
  "Temperature: ",
  { "text": "42°C", "fg": "red", "bold": true }
]
```

---

### `paragraph`

Multi-line styled text.

```json
{
  "id": "title",
  "type": "paragraph",
  "text": [
    { "text": "Dashboard", "fg": "cyan", "bold": true },
    " — powered by Jotui"
  ],
  "align": "center",
  "wrap": true,
  "border": "rounded",
  "title": "Header"
}
```

| Property | Type         | Default  | Description                 |
|----------|--------------|----------|-----------------------------|
| `text`   | string/array | `""`     | Plain string or span array  |
| `align`  | string       | `"left"` | `"left"`, `"center"`, `"right"` |
| `wrap`   | boolean      | `true`   | Enable word wrapping        |

Default border: `"none"`

---

### `list`

Selectable, scrollable list.

```json
{
  "id": "logs",
  "type": "list",
  "items": [
    "Boot complete",
    "Network ready",
    [{ "text": "Error: ", "fg": "red" }, "disk full"]
  ],
  "selected": 0,
  "scrollbar": true,
  "focusable": true,
  "highlight_symbol": "▶ ",
  "highlight_style": { "fg": "black", "bg": "cyan", "bold": true },
  "border": "rounded",
  "title": "System Logs"
}
```

| Property           | Type     | Default                                          |
|--------------------|----------|--------------------------------------------------|
| `items`            | array    | `[]` — strings or span arrays                    |
| `selected`         | int/null | `null`                                           |
| `scrollbar`        | boolean  | `false`                                          |
| `focusable`        | boolean  | `true`                                           |
| `highlight_symbol` | string   | `">> "`                                          |
| `highlight_style`  | object   | `{ "fg": "black", "bg": "white", "bold": true }` |

Default border: `"rounded"`

---

### `table`

Tabular data with headers and selectable rows.

```json
{
  "id": "processes",
  "type": "table",
  "headers": ["PID", "Name", "CPU %", "Status"],
  "rows": [
    ["1", "systemd", "0.1", "running"],
    ["512", "nginx", "2.3", "running"]
  ],
  "widths": ["10%", "30%", "15%", "*"],
  "selected": 0,
  "focusable": true,
  "highlight_style": { "fg": "black", "bg": "magenta" },
  "header_style": { "fg": "yellow", "bold": true },
  "border": "rounded",
  "title": "Processes"
}
```

| Property          | Type     | Default                                          |
|-------------------|----------|--------------------------------------------------|
| `headers`         | array    | `[]` — array of strings                          |
| `rows`            | array    | `[]` — array of string arrays                    |
| `widths`          | array    | `[]` — constraint strings (same as layout `size`). Empty = even distribution |
| `selected`        | int/null | `null`                                           |
| `scrollbar`       | boolean  | `false`                                          |
| `focusable`       | boolean  | `true`                                           |
| `highlight_style` | object   | `{ "fg": "black", "bg": "white" }`               |
| `header_style`    | object   | `{ "fg": "yellow", "bold": true }`                |

Default border: `"rounded"`

---

### `tabs`

Tab bar for navigation.

```json
{
  "id": "nav",
  "type": "tabs",
  "titles": ["Dashboard", "Settings", "Logs"],
  "selected": 0,
  "focusable": true,
  "highlight_style": { "fg": "cyan", "bold": true },
  "divider": " | "
}
```

| Property          | Type    | Default                                          |
|-------------------|---------|--------------------------------------------------|
| `titles`          | array   | `[]` — array of strings                          |
| `selected`        | int     | `0`                                              |
| `focusable`       | boolean | `true`                                           |
| `highlight_style` | object  | `{ "fg": "yellow", "bold": true }`                |
| `divider`         | string  | `" \| "`                                         |

Default border: `"none"`

---

### `gauge`

Progress bar with label.

```json
{
  "id": "cpu",
  "type": "gauge",
  "value": 72,
  "max": 100,
  "label": "CPU",
  "style": { "fg": "green" },
  "border": "rounded",
  "title": "CPU Usage"
}
```

| Property | Type   | Default |
|----------|--------|---------|
| `value`  | int    | `0`     |
| `max`    | int    | `100`   |
| `label`  | string | `""`    |

Label displays as `"CPU: 72%"`. If label is empty, just `"72%"`.

Default border: `"rounded"`, default style fg: `"green"`

---

### `line_gauge`

Thin line-style progress indicator. Same properties as `gauge`.

Default border: `"none"`, default style fg: `"green"`

---

### `sparkline`

Miniature bar chart from a data array.

```json
{
  "id": "cpu_history",
  "type": "sparkline",
  "data": [10, 20, 30, 25, 40, 35, 50, 45],
  "max": 100,
  "style": { "fg": "green" },
  "border": "rounded",
  "title": "CPU History"
}
```

| Property | Type     | Default |
|----------|----------|---------|
| `data`   | array    | `[]` — array of integers |
| `max`    | int/null | `null` — auto-scaled if null |

Default border: `"none"`, default style fg: `"green"`

---

### `bar_chart`

Vertical bar chart.

```json
{
  "id": "services",
  "type": "bar_chart",
  "bars": [
    ["web", 82],
    ["api", 64],
    ["db", 45]
  ],
  "bar_width": 5,
  "max": 100,
  "border": "rounded",
  "title": "Service Load (%)"
}
```

| Property    | Type     | Default |
|-------------|----------|---------|
| `bars`      | array    | `[]` — array of `[label, value]` tuples |
| `bar_width` | int      | `3`     |
| `max`       | int/null | `null` — auto-scaled if null |

Default border: `"rounded"`

---

## Focus & Keyboard

Jotui manages focus internally. Widgets with `"focusable": true` are collected into a focus ring (depth-first layout order).

| Key         | Action                                  |
|-------------|-----------------------------------------|
| `Tab`       | Next focusable widget                   |
| `Shift+Tab` | Previous focusable widget              |
| `↑` / `↓`  | Select prev/next item in list/table     |
| `←` / `→`  | Switch tab in tabs widget               |
| `Enter`     | Confirm selection (emits event)         |
| `Ctrl+Q`   | Quit                                    |

The focused widget gets a highlighted border (cyan, bold) automatically.

---

## Three-Layer Merge

Widget state is computed by merging three layers:

```
Layer 1: defaults.json     → base defaults per widget type
Layer 2: render message    → backend's initial definition
Layer 3: patch messages    → incremental updates
```

Each layer shallow-merges on top of the previous. Nested objects (like `style`) merge one level deep.

```
defaults:  { value: 0, max: 100, style: { fg: "green", bg: "reset" } }
render:    { value: 45, label: "Temp" }
patch:     { value: 87, style: { fg: "red" } }
─────────────────────────────────────────────────────
final:     { value: 87, max: 100, label: "Temp", style: { fg: "red", bg: "reset" } }
```

---

## Minimal Example

The smallest valid Jotui program (Python):

```python
import json, socket, subprocess

proc = subprocess.Popen(
    ["cargo", "run", "--quiet"],
    stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=None
)

# Read port from Jotui's first stdout line
port = json.loads(proc.stdout.readline())["port"]
proc.stdout.close()

# Connect via TCP
sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.connect(("127.0.0.1", port))

# Send render message
msg = {"jsonrpc": "2.0", "method": "render", "params": {
    "pages": {"main": {
        "layout": {"ref": "hello"},
        "widgets": [{"id": "hello", "type": "paragraph", "text": "Hello, world!"}]
    }},
    "active": "main"
}}
body = json.dumps(msg)
sock.sendall(f"Content-Length: {len(body)}\r\n\r\n".encode() + body.encode())
proc.wait()
```

---

## Error Handling

Jotui never crashes on bad input. All errors are logged to stderr.

| Error                         | Behavior                            |
|-------------------------------|-------------------------------------|
| Invalid JSON body             | Ignored, logged                     |
| Missing Content-Length header | Ignored, logged                     |
| Unknown method                | Ignored, logged                     |
| Unknown widget `type`         | Widget skipped, logged              |
| Patch targets missing page    | Ignored, logged                     |
| Patch targets missing widget  | That update skipped, logged         |
| Navigate to missing page      | Stays on current page, logged       |
| Style reference not found     | Falls back to widget type defaults  |
