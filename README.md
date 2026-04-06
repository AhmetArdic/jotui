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
1. Your backend listens on a TCP port
2. Your backend spawns Jotui with `--port <PORT>`
3. Jotui connects to the backend's TCP port
4. Communication happens over JSON-RPC 2.0 with Content-Length framing

---

## Protocol — JSON-RPC 2.0

All communication uses [JSON-RPC 2.0](https://www.jsonrpc.org/specification) notifications with **Content-Length framing** (same as LSP):

```
Content-Length: 84\r\n
\r\n
{"jsonrpc":"2.0","method":"render","params":{...}}
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

Sent once at startup. Contains all pages as a unified tree — layout and widgets are defined together.

```json
{
  "jsonrpc": "2.0",
  "method": "render",
  "params": {
    "defs": {
      "danger": { "fg": "red", "bold": true },
      "ok": { "fg": "green" },
      "std_border": "rounded"
    },
    "pages": [
      {
        "id": "dashboard",
        "children": [
          { "size": "3", "type": "paragraph", "id": "header", "text": "My App" },
          {
            "dir": "h",
            "children": [
              { "size": "30%", "type": "list", "id": "sidebar", "items": ["Home", "Settings"] },
              { "type": "paragraph", "id": "main", "text": "Content here" }
            ]
          },
          { "size": "1", "type": "paragraph", "id": "footer", "text": "Status: OK" }
        ]
      },
      {
        "id": "settings",
        "children": [ "..." ]
      }
    ],
    "active": "dashboard"
  }
}
```

**Params fields:**

| Field    | Type   | Required | Description                                          |
|----------|--------|----------|------------------------------------------------------|
| `defs`   | object | no       | Named definitions for reuse via `$name` references   |
| `pages`  | array  | yes      | Ordered array of page definitions                    |
| `active` | string | no       | Initially active page. Defaults to first page        |

### `patch` — Update widgets

Sent anytime after render. Only include changed properties — everything else is preserved.

```json
{
  "jsonrpc": "2.0",
  "method": "patch",
  "params": {
    "page": "dashboard",
    "updates": [
      { "id": "cpu_gauge", "value": 87, "style": "$danger" },
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
    "action": "submit",
    "value": 2
  }
}
```

There are only two event actions:

| Action   | When                              | Value                                              |
|----------|-----------------------------------|----------------------------------------------------|
| `submit` | Enter pressed on any widget       | depends on widget: index (list/table), string (input/tabs), null (other) |
| `key`    | Any other key press               | null (key name in `key` field)                     |

The backend determines context from the `source` field (widget ID).

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

## Unified Tree

Each page is a tree where **layout and widgets are defined together**. A node with `type` is a widget (leaf). A node with `children` is a layout container. There is no separate `layout` / `widgets` split.

```json
{
  "id": "dashboard",
  "children": [
    { "size": "3", "type": "paragraph", "id": "header", "text": "My App" },
    {
      "dir": "h",
      "children": [
        { "size": "30%", "type": "list", "id": "sidebar", "items": ["Home", "Settings"] },
        { "type": "paragraph", "id": "main", "text": "Content here" }
      ]
    },
    { "size": "1", "type": "paragraph", "id": "footer", "text": "Status: OK" }
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

### Container Nodes

Containers have `children` and control layout direction.

| Field      | Type    | Default | Description                              |
|------------|---------|---------|------------------------------------------|
| `dir`      | string  | `"v"`   | `"v"` (vertical) or `"h"` (horizontal)  |
| `size`     | string  | `"*"`   | Size constraint (see below)              |
| `margin`   | integer | `0`     | Margin in cells around this node         |
| `children` | array   | —       | Child nodes                              |

### Widget Nodes

Widgets have `type` and are the leaf nodes of the tree.

| Field  | Type   | Required | Description                              |
|--------|--------|----------|------------------------------------------|
| `type` | string | yes      | Widget type name                         |
| `id`   | string | yes      | Unique within the page (needed for patch and events) |
| `size` | string | `"*"`    | Size constraint (see below)              |

Plus widget-specific properties (see Widgets section).

### Size Constraints

| Value   | Meaning                    | Example              |
|---------|----------------------------|----------------------|
| `"*"`   | Fill remaining space       | `{ "type": "paragraph", "id": "main", "text": "..." }` |
| `"50%"` | Percentage of parent       | `{ "size": "50%", "type": "list", ... }` |
| `"3"`   | Fixed number of rows/cols  | `{ "size": "3", "type": "paragraph", ... }` |
| `">5"`  | Minimum 5 cells            | `{ "size": ">5", ... }` |
| `"<20"` | Maximum 20 cells           | `{ "size": "<20", ... }` |

---

## Defs — Reusable Definitions

The `defs` object lets you define values once and reference them anywhere in widget properties using `$name` syntax.

```json
{
  "defs": {
    "danger": { "fg": "red", "bold": true },
    "ok": { "fg": "green" },
    "header": { "fg": "cyan", "bold": true },
    "muted": { "fg": "dark_gray" }
  }
}
```

Use with `$` prefix in any top-level widget property:

```json
{ "type": "gauge", "id": "cpu", "value": 72, "style": "$ok", "highlight_style": "$header" }
```

```json
{ "type": "paragraph", "id": "status", "text": "OK", "style": "$muted" }
```

`$` references work in both `render` and `patch` messages. Any string value starting with `$` is looked up in `defs` and replaced with the corresponding value. If the reference is not found, the string is kept as-is.

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
| `style`   | string/object | varies  | Style object or `$name` reference to `defs`       |

### Style Object

```json
{ "fg": "cyan", "bg": "black", "bold": true, "italic": false, "underline": false }
```

**Colors:** `"red"`, `"green"`, `"blue"`, `"yellow"`, `"cyan"`, `"magenta"`, `"gray"`, `"dark_gray"`, `"white"`, `"black"`, `"reset"`, `"#FF5500"` (hex), `"color(214)"` (256-palette)

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
  "type": "paragraph", "id": "title",
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
  "type": "list", "id": "logs",
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
  "type": "table", "id": "processes",
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
| `widths`          | array    | `[]` — constraint strings (same as `size`). Empty = even distribution |
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
  "type": "tabs", "id": "nav",
  "titles": ["Dashboard", "Settings", "Logs"],
  "selected": 0,
  "focusable": true,
  "highlight_style": "$header",
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
  "type": "gauge", "id": "cpu",
  "value": 72, "max": 100, "label": "CPU",
  "style": "$ok",
  "border": "rounded", "title": "CPU Usage"
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
  "type": "sparkline", "id": "cpu_history",
  "data": [10, 20, 30, 25, 40, 35, 50, 45],
  "max": 100,
  "style": "$ok",
  "border": "rounded", "title": "CPU History"
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
  "type": "bar_chart", "id": "services",
  "bars": [
    ["web", 82],
    ["api", 64],
    ["db", 45]
  ],
  "bar_width": 5, "max": 100,
  "border": "rounded", "title": "Service Load (%)"
}
```

| Property    | Type     | Default |
|-------------|----------|---------|
| `bars`      | array    | `[]` — array of `[label, value]` tuples |
| `bar_width` | int      | `3`     |
| `max`       | int/null | `null` — auto-scaled if null |

Default border: `"rounded"`

---

### `input`

Text input box with local editing. When focused, character keys type into the field.

```json
{
  "type": "input", "id": "cmd",
  "value": "",
  "placeholder": "Type a command...",
  "cursor": 0,
  "focusable": true,
  "border": "rounded", "title": "Command"
}
```

| Property      | Type    | Default   |
|---------------|---------|-----------|
| `value`       | string  | `""`      |
| `placeholder` | string  | `""`      |
| `cursor`      | int     | `0`       |
| `focusable`   | boolean | `true`    |

**Keyboard (when focused):** type characters, Backspace/Delete, Left/Right/Home/End for cursor, Enter to submit. Tab/Shift+Tab still changes focus.

On Enter, emits `submit` with `value` set to the current text string.

Default border: `"rounded"`

---

### `chart`

Line/scatter chart. Ideal for time-series data (ADC, sensors, metrics).

```json
{
  "type": "chart", "id": "adc",
  "datasets": [
    {
      "name": "CH0",
      "data": [[0, 512], [1, 520], [2, 498]],
      "style": { "fg": "cyan" },
      "marker": "braille",
      "graph_type": "line"
    }
  ],
  "x_axis": { "title": "Time (s)", "bounds": [0, 100] },
  "y_axis": { "title": "Value", "bounds": [0, 1024] },
  "border": "rounded", "title": "ADC"
}
```

| Property   | Type   | Default                              |
|------------|--------|--------------------------------------|
| `datasets` | array  | `[]` — array of dataset objects      |
| `x_axis`   | object | `{ "title": "", "bounds": [0, 100] }` |
| `y_axis`   | object | `{ "title": "", "bounds": [0, 100] }` |

**Dataset object:**

| Field        | Type   | Default     | Description                                |
|--------------|--------|-------------|--------------------------------------------|
| `name`       | string | `""`        | Legend label                               |
| `data`       | array  | `[]`        | Array of `[x, y]` number pairs             |
| `style`      | object | cyan fg     | Line/dot color                             |
| `marker`     | string | `"braille"` | `"braille"`, `"dot"`, `"block"`, `"bar"`, `"halfblock"` |
| `graph_type` | string | `"line"`    | `"line"` or `"scatter"`                    |

**Axis object:**

| Field    | Type   | Default    | Description                              |
|----------|--------|------------|------------------------------------------|
| `title`  | string | `""`       | Axis label                               |
| `bounds` | array  | `[0, 100]` | `[min, max]` range                       |
| `labels` | array  | auto       | Custom tick labels. Auto-generated from bounds if omitted |
| `style`  | object | inherited  | Axis style                               |

Default border: `"rounded"`

---

## Focus & Keyboard

Jotui manages focus internally. Widgets with `"focusable": true` are collected into a focus ring (depth-first tree order).

| Key         | Action                                            |
|-------------|---------------------------------------------------|
| `Tab`       | Next focusable widget                             |
| `Shift+Tab` | Previous focusable widget                        |
| `↑` / `↓`  | Select prev/next item in list/table               |
| `←` / `→`  | Switch tab in tabs widget, move cursor in input   |
| `Enter`     | Submit (emits `submit` event)                     |
| `Ctrl+Q`   | Quit                                              |
| Characters  | Type into focused input widget                    |
| `Backspace` | Delete character in input widget                  |

The focused widget gets a highlighted border (cyan, bold) automatically.

---

## Three-Layer Merge

Widget state is computed by merging three layers:

```
Layer 1: defaults.json     → base defaults per widget type
Layer 2: render message    → backend's initial definition
Layer 3: patch messages    → incremental updates
```

Each layer shallow-merges on top of the previous. Nested objects (like `style`) merge one level deep. `$name` references are resolved before merging.

```
defaults:  { value: 0, max: 100, style: { fg: "green", bg: "reset" } }
render:    { value: 45, label: "Temp", style: "$ok" }
patch:     { value: 87, style: "$danger" }
─────────────────────────────────────────────────────
final:     { value: 87, max: 100, label: "Temp", style: { fg: "red", bold: true } }
```

---

## Minimal Example

The smallest valid Jotui program (Python):

```python
import json, socket, subprocess, sys

# Backend listens on TCP
server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
server.bind(("127.0.0.1", 0))
server.listen(1)
port = server.getsockname()[1]

# Spawn Jotui as subprocess
proc = subprocess.Popen(["cargo", "run", "--quiet", "--", "--port", str(port)])

# Wait for connection
conn, _ = server.accept()
server.close()

# Send render message
msg = json.dumps({"jsonrpc": "2.0", "method": "render", "params": {
    "pages": [{"id": "main", "children": [
        {"type": "paragraph", "id": "hello", "text": "Hello, world!"}
    ]}]
}})
conn.sendall(f"Content-Length: {len(msg)}\r\n\r\n{msg}".encode())
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
| `$name` ref not found         | String kept as-is, uses defaults    |
