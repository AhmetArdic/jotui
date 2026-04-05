# ratatui-json-ui — Specification v1.0

## Purpose

This document is the single source of truth for implementing **ratatui-json-ui**: a JSON-driven terminal UI engine built on Ratatui. It is written for LLM-assisted implementation. Every architectural decision, data format, merge rule, and behavioral contract is defined here. If something is not in this document, it is not part of the design.

---

## 1. Architecture Overview

```
┌──────────┐    JSON     ┌──────────────┐   Ratatui    ┌──────────┐
│ Backend  │ ──────────→ │   Frontend   │ ──────────→  │ Terminal │
│ (any     │ ←────────── │   Engine     │              │          │
│  language)│    Event   │   (Rust)     │              │          │
└──────────┘             └──────────────┘              └──────────┘
```

**Backend**: Any language/runtime. Produces JSON messages describing UI. Owns all business logic.

**Frontend Engine**: Rust binary using Ratatui + Crossterm. Receives JSON, renders terminal UI, sends user interaction events back as JSON. Has zero business logic. It is a "dumb renderer" that only knows how to draw widgets described by JSON.

**Transport Layer**: Abstracted behind a trait. First implementation uses stdin/stdout. The engine does not care how JSON arrives — stdin, TCP, serial, shared memory are all valid. As long as JSON is delivered, the frontend works.

---

## 2. Core Concepts

### 2.1 Server-Driven UI

The backend decides what the user sees. The frontend never decides which page to show, which widgets to display, or what content to render. Every visual decision comes from JSON produced by the backend.

### 2.2 Immediate Mode + State Store

Ratatui is an immediate-mode renderer — it redraws every frame from scratch and retains no state. The frontend engine adds a **State Store** on top: a `HashMap` that holds the last-known state of every widget on every page. Each frame, the renderer reads from the State Store and draws.

### 2.3 Three-Layer Merge

Every widget's final state is computed by merging three layers:

```
Layer 1: defaults.json     (base defaults per widget type)
Layer 2: render message     (backend's initial definition)
Layer 3: patch messages     (incremental updates)
```

Each layer shallow-merges on top of the previous one. Nested objects (e.g., `style`) are also shallow-merged one level deep.

**Example:**

```
defaults.json gauge:  { value: 0, max: 100, style: { fg: "green", bg: "reset" } }
render props:         { value: 45, label: "Temp" }
patch:                { value: 87, style: { fg: "red" } }

final state:          { value: 87, max: 100, label: "Temp", style: { fg: "red", bg: "reset" } }
```

Note: `style.bg` is preserved from defaults because patch only overrode `style.fg`.

---

## 3. Message Protocol

Four message types. Three flow backend → frontend, one flows frontend → backend.

### 3.1 `render` (Backend → Frontend)

Sent once at startup. Contains ALL pages, ALL widgets, a shared style dictionary, and which page is active. The frontend loads everything into memory.

```json
{
  "msg": "render",
  "styles": {
    "danger": { "fg": "red", "bold": true },
    "ok": { "fg": "green" },
    "header": { "fg": "yellow", "bold": true }
  },
  "page_order": ["dashboard", "settings"],
  "pages": {
    "dashboard": {
      "layout": {
        "children": [
          { "size": "3", "ref": "title" },
          {
            "dir": "h",
            "children": [
              { "size": "50%", "ref": "temp" },
              { "ref": "logs" }
            ]
          },
          { "size": "1", "ref": "status" }
        ]
      },
      "widgets": [
        { "id": "title", "type": "paragraph", "text": "Dashboard", "style": "header" },
        { "id": "temp", "type": "gauge", "value": 45, "label": "Sıcaklık" },
        { "id": "logs", "type": "list", "items": ["Boot OK", "Sensor ready"] },
        { "id": "status", "type": "paragraph", "text": "Connected", "style": "ok" }
      ]
    },
    "settings": {
      "layout": {
        "children": [
          { "ref": "lbl" }
        ]
      },
      "widgets": [
        { "id": "lbl", "type": "paragraph", "text": "Settings" }
      ]
    }
  },
  "active": "dashboard"
}
```

**Rules:**

- `styles` is optional. If present, widgets can reference a style by name (string) instead of inline object.
- `page_order` defines the canonical ordering of pages (useful if tabs are shown). JSON object key order is not guaranteed, so this array provides explicit ordering.
- `pages` is an object keyed by page ID. Each page has `layout` and `widgets`.
- `active` sets which page is displayed initially.
- Widget `id` values must be unique within a page. They do not need to be globally unique across pages.

### 3.2 `patch` (Backend → Frontend)

Sent anytime after initial render. Updates specific widget properties on a specific page. Only changed properties are included — everything else is preserved via shallow merge.

```json
{
  "msg": "patch",
  "page": "dashboard",
  "updates": [
    { "id": "temp", "value": 87, "style": "danger" },
    { "id": "status", "text": "Warning!" }
  ]
}
```

**Rules:**

- `page` identifies the target page. The page does NOT need to be the active page. Backend can update background pages.
- Each entry in `updates` must include `id`. All other fields are optional and override existing values via shallow merge.
- `type` cannot be changed via patch. To change a widget's type, resend a full render for that page.
- If `id` does not exist on the target page, the update is silently ignored (logged to stderr).
- Patch cannot add new widgets or remove existing widgets. Use full render for structural changes.

### 3.3 `navigate` (Backend → Frontend)

Switches the active page. No round-trip — the page is already in memory.

```json
{
  "msg": "navigate",
  "page": "settings"
}
```

**Rules:**

- If `page` does not exist, the message is silently ignored (logged to stderr).
- The previous page's state (including focus position) is preserved.

### 3.4 `event` (Frontend → Backend)

Sent when the user interacts with a widget or presses a key.

```json
{
  "msg": "event",
  "page": "dashboard",
  "source": "logs",
  "action": "select",
  "value": 2
}
```

**Rules:**

- `page` is always the currently active page.
- `source` is the widget ID. For global keys (not tied to a widget), `source` is `null`.
- `value` type depends on context: integer for selection index, string for text input, etc.

**Action types (exhaustive list):**

| Action   | Meaning                          | Typical value         |
|----------|----------------------------------|-----------------------|
| `select` | Item selected in list/table      | Integer (item index)  |
| `submit` | Enter/confirm pressed            | String or null        |
| `change` | Tab changed, value modified      | Integer or string     |
| `key`    | Key press (global or widget)     | null (`key` field has the key name) |

For `key` action, an additional `key` field contains the key name:

```json
{
  "msg": "event",
  "page": "dashboard",
  "source": null,
  "action": "key",
  "key": "f1"
}
```

**Events that are NOT sent to backend:**

- Scroll (viewport management is frontend-internal)
- Focus/blur (focus tracking is frontend-internal)
- Resize (frontend handles re-layout internally)

---

## 4. Layout System

Layout is a recursive tree. Each node is either a **container** (has `children`) or a **leaf** (has `ref` pointing to a widget ID).

### 4.1 Layout Node Structure

```json
{
  "dir": "v",
  "size": "*",
  "margin": 0,
  "border": "none",
  "title": "",
  "children": [
    { "size": "30%", "ref": "sidebar" },
    { "ref": "main" }
  ]
}
```

**Fields:**

| Field      | Type     | Default    | Description                                  |
|------------|----------|------------|----------------------------------------------|
| `dir`      | string   | `"v"`      | Split direction: `"v"` (vertical), `"h"` (horizontal) |
| `size`     | string   | `"*"`      | Size constraint (see below)                  |
| `margin`   | integer  | `0`        | Margin in cells around this node             |
| `border`   | string   | `"none"`   | Border type around this container            |
| `title`    | string   | `""`       | Title shown in border                        |
| `children` | array    | —          | Child nodes (present only on containers)     |
| `ref`      | string   | —          | Widget ID (present only on leaves)           |

A node has EITHER `children` OR `ref`, never both.

### 4.2 Size Constraints

| JSON value | Ratatui Constraint  | Meaning                          |
|------------|---------------------|----------------------------------|
| `"*"`      | `Min(0)`            | Fill remaining space             |
| `"30%"`    | `Percentage(30)`    | Percentage of parent             |
| `"5"`      | `Length(5)`         | Fixed number of cells            |
| `">3"`     | `Min(3)`            | At least 3 cells                 |
| `"<10"`    | `Max(10)`           | At most 10 cells                 |

If `size` is omitted, default is `"*"` (fill remaining space).

### 4.3 Layout Example

A classic dashboard layout:

```json
{
  "layout": {
    "children": [
      { "size": "3", "ref": "header" },
      {
        "dir": "h",
        "children": [
          { "size": "25%", "ref": "sidebar" },
          {
            "children": [
              { "ref": "main_content" },
              { "size": "10", "ref": "detail_panel" }
            ]
          }
        ]
      },
      { "size": "1", "ref": "footer" }
    ]
  }
}
```

This produces:

```
┌─────────────────────────────────────┐
│ header (3 rows)                     │
├──────────┬──────────────────────────┤
│ sidebar  │ main_content (fill)      │
│ (25%)    │                          │
│          ├──────────────────────────┤
│          │ detail_panel (10 rows)   │
├──────────┴──────────────────────────┤
│ footer (1 row)                      │
└─────────────────────────────────────┘
```

---

## 5. Widget Catalog

### 5.1 Common Properties

Every widget has these properties. They do not need to be listed in each widget definition below.

| Property    | Type          | Default    | Description                              |
|-------------|---------------|------------|------------------------------------------|
| `id`        | string        | (required) | Unique identifier within the page        |
| `type`      | string        | (required) | Widget type name                         |
| `visible`   | boolean       | `true`     | If false, widget is not rendered but its layout space is preserved (blank area) |
| `border`    | string        | varies     | `"none"`, `"plain"`, `"rounded"`, `"double"`, `"thick"` |
| `title`     | string        | `""`       | Title displayed in border. Ignored if border is `"none"` |
| `style`     | string/object | varies     | Inline style object OR string reference to `styles` dictionary |

### 5.2 Style Object

```json
{
  "fg": "white",
  "bg": "reset",
  "bold": false,
  "italic": false,
  "underline": false
}
```

**Color values:**

- Named: `"red"`, `"green"`, `"blue"`, `"yellow"`, `"cyan"`, `"magenta"`, `"gray"`, `"dark_gray"`, `"white"`, `"black"`, `"reset"`
- RGB hex: `"#FF5500"`
- 256 palette: `"color(214)"`

When `style` is a string (e.g., `"danger"`), it references an entry in the `styles` dictionary from the `render` message. The referenced style object is merged on top of the widget type's default style.

### 5.3 Styled Text

The `text` field on `paragraph` widgets (and any other text field) accepts two formats:

**Plain string:**

```json
{ "text": "Hello world" }
```

**Span array (inline styling):**

```json
{
  "text": [
    "Temperature: ",
    { "text": "42°C", "fg": "red", "bold": true }
  ]
}
```

Array elements can be plain strings (rendered with widget's default style) or objects with `text` plus any style properties (`fg`, `bg`, `bold`, `italic`, `underline`).

### 5.4 Widget Type Definitions

#### `paragraph`

Multi-line styled text display.

| Property | Type          | Default  |
|----------|---------------|----------|
| `text`   | string/array  | `""`     |
| `align`  | string        | `"left"` |
| `wrap`   | boolean       | `true`   |

`align`: `"left"`, `"center"`, `"right"`

Default `border`: `"none"`

#### `list`

Selectable list of items.

| Property           | Type     | Default                                      |
|--------------------|----------|----------------------------------------------|
| `items`            | array    | `[]`                                         |
| `selected`         | int/null | `null`                                       |
| `scrollbar`        | boolean  | `false`                                      |
| `focusable`        | boolean  | `true`                                       |
| `highlight_symbol` | string   | `">> "`                                      |
| `highlight_style`  | object   | `{ "fg": "black", "bg": "white", "bold": true }` |

`items`: Array of strings or span arrays (same format as `text`).

Default `border`: `"rounded"`

#### `table`

Tabular data with headers.

| Property        | Type     | Default                                          |
|-----------------|----------|--------------------------------------------------|
| `headers`       | array    | `[]`                                             |
| `rows`          | array    | `[]`                                             |
| `widths`        | array    | `[]`                                             |
| `selected`      | int/null | `null`                                           |
| `scrollbar`     | boolean  | `false`                                          |
| `focusable`     | boolean  | `true`                                           |
| `highlight_style` | object | `{ "fg": "black", "bg": "white" }`               |
| `header_style`  | object   | `{ "fg": "yellow", "bg": "reset", "bold": true }` |

`headers`: Array of strings.  
`rows`: Array of arrays of strings. Each inner array is one row.  
`widths`: Array of constraint strings (same format as layout `size`). If empty, columns are evenly distributed.

Default `border`: `"rounded"`

#### `tabs`

Tab bar for navigation display.

| Property          | Type   | Default                                          |
|-------------------|--------|--------------------------------------------------|
| `titles`          | array  | `[]`                                             |
| `selected`        | int    | `0`                                              |
| `focusable`       | boolean| `true`                                           |
| `highlight_style` | object | `{ "fg": "yellow", "bg": "reset", "bold": true }` |
| `divider`         | string | `" \| "`                                         |

Default `border`: `"none"`

#### `gauge`

Filled progress bar with label.

| Property | Type   | Default |
|----------|--------|---------|
| `value`  | int    | `0`     |
| `max`    | int    | `100`   |
| `label`  | string | `""`    |

Default `border`: `"rounded"`  
Default `style.fg`: `"green"`

#### `line_gauge`

Thin line-style progress indicator.

| Property   | Type   | Default    |
|------------|--------|------------|
| `value`    | int    | `0`        |
| `max`      | int    | `100`      |
| `label`    | string | `""`       |
| `line_set` | string | `"normal"` |

`line_set`: `"normal"`, `"thick"`, `"double"`

Default `border`: `"none"`  
Default `style.fg`: `"green"`

#### `sparkline`

Miniature line chart from data array.

| Property | Type     | Default |
|----------|----------|---------|
| `data`   | array    | `[]`    |
| `max`    | int/null | `null`  |

`data`: Array of integers.  
`max`: If null, auto-scaled to data maximum.

Default `border`: `"none"`  
Default `style.fg`: `"green"`

#### `bar_chart`

Vertical bar chart.

| Property    | Type     | Default |
|-------------|----------|---------|
| `bars`      | array    | `[]`    |
| `max`       | int/null | `null`  |
| `bar_width` | int      | `3`     |

`bars`: Array of `[label, value]` tuples. Example: `[["CPU", 82], ["RAM", 64]]`  
`max`: If null, auto-scaled to data maximum.

Default `border`: `"rounded"`

---

## 6. defaults.json

This file defines the default property values for every layout node and widget type. The frontend loads it at startup. Any property not specified in a render or patch message falls back to these values.

```json
{
  "layout": {
    "dir": "v",
    "size": "*",
    "margin": 0,
    "border": "none",
    "title": ""
  },

  "paragraph": {
    "text": "",
    "align": "left",
    "wrap": true,
    "border": "none",
    "title": "",
    "visible": true,
    "style": { "fg": "white", "bg": "reset", "bold": false, "italic": false, "underline": false }
  },

  "list": {
    "items": [],
    "selected": null,
    "scrollbar": false,
    "border": "rounded",
    "title": "",
    "visible": true,
    "focusable": true,
    "highlight_symbol": ">> ",
    "highlight_style": { "fg": "black", "bg": "white", "bold": true },
    "style": { "fg": "white", "bg": "reset" }
  },

  "table": {
    "headers": [],
    "rows": [],
    "widths": [],
    "selected": null,
    "scrollbar": false,
    "border": "rounded",
    "title": "",
    "visible": true,
    "focusable": true,
    "highlight_style": { "fg": "black", "bg": "white" },
    "header_style": { "fg": "yellow", "bg": "reset", "bold": true },
    "style": { "fg": "white", "bg": "reset" }
  },

  "tabs": {
    "titles": [],
    "selected": 0,
    "border": "none",
    "title": "",
    "visible": true,
    "focusable": true,
    "highlight_style": { "fg": "yellow", "bg": "reset", "bold": true },
    "divider": " | ",
    "style": { "fg": "white", "bg": "reset" }
  },

  "gauge": {
    "value": 0,
    "max": 100,
    "label": "",
    "border": "rounded",
    "title": "",
    "visible": true,
    "style": { "fg": "green", "bg": "reset" }
  },

  "line_gauge": {
    "value": 0,
    "max": 100,
    "label": "",
    "border": "none",
    "title": "",
    "visible": true,
    "style": { "fg": "green", "bg": "reset" }
  },

  "sparkline": {
    "data": [],
    "max": null,
    "border": "none",
    "title": "",
    "visible": true,
    "style": { "fg": "green", "bg": "reset" }
  },

  "bar_chart": {
    "bars": [],
    "max": null,
    "bar_width": 3,
    "border": "rounded",
    "title": "",
    "visible": true,
    "style": { "fg": "white", "bg": "reset" }
  }
}
```

---

## 7. Page Management

### 7.1 Multi-Page Architecture

All pages are sent in the initial `render` message and stored in memory. Page switching via `navigate` is instant — no round-trip to backend.

### 7.2 State Store Structure

```
HashMap<PageID, Page>

Page {
    layout: LayoutTree,
    widgets: HashMap<WidgetID, WidgetState>,
    focus_index: Option<usize>
}
```

### 7.3 Background Updates

Backend can send `patch` messages targeting any page, including non-active pages. The state is updated immediately. When the user navigates to that page, the latest state is already ready.

### 7.4 Page Re-render

If backend needs to structurally change a page (add/remove widgets, change layout), it sends a new `render` message. This replaces all pages and resets state. There is no per-page re-render — `render` is always a complete replacement.

---

## 8. Focus System

### 8.1 Focus Ring

Each page has an independent focus ring. The ring is built by collecting all widgets with `focusable: true` in layout-tree depth-first order. No explicit `focus_order` property is needed.

### 8.2 Navigation

- **Tab**: Move focus to next widget in ring
- **Shift+Tab**: Move focus to previous widget in ring
- Focus wraps around at both ends

### 8.3 Visual Indicator

The focused widget receives a visual indicator — typically a highlighted border color. This is applied by the frontend renderer automatically. The exact visual treatment is a frontend implementation detail.

### 8.4 Cross-Page Focus

Each page's focus position is preserved independently. Navigating away from a page and back restores the previous focus position.

---

## 9. Border System

| JSON value  | Ratatui mapping        | Visual            |
|-------------|------------------------|--------------------|
| `"none"`    | No Block wrapper       | No border drawn    |
| `"plain"`   | `BorderType::Plain`    | `+-+\|  \|+-+`    |
| `"rounded"` | `BorderType::Rounded`  | `╭─╮│  │╰─╯`      |
| `"double"`  | `BorderType::Double`   | `╔═╗║  ║╚═╝`      |
| `"thick"`   | `BorderType::Thick`    | `┏━┓┃  ┃┗━┛`      |

When `border` is `"none"`, the widget is rendered without a `Block` wrapper. When any other value is set, a `Block` with that border type wraps the widget. If `title` is non-empty, it is displayed in the top border.

---

## 10. Error Handling Policy

The frontend never crashes due to bad input. All errors are handled silently with stderr logging.

| Error condition                      | Frontend behavior                       |
|--------------------------------------|-----------------------------------------|
| Invalid JSON                         | Ignore message, log to stderr           |
| Unknown `msg` type                   | Ignore message, log to stderr           |
| Unknown widget `type`                | Skip widget, log to stderr              |
| Patch targets nonexistent page       | Ignore patch, log to stderr             |
| Patch targets nonexistent widget ID  | Ignore that update entry, log to stderr |
| Navigate to nonexistent page         | Ignore, stay on current page, log       |
| Style reference not found in dictionary | Fall back to widget type defaults, log |

---

## 11. Implementation Plan

### Phase 1 — Skeleton

- Initialize Rust project with dependencies: `ratatui`, `crossterm`, `serde`, `serde_json`
- Implement Defaults Manager: parse and store `defaults.json` at startup
- Implement JSON message parser: deserialize incoming JSON, route by `msg` field
- Implement Page Store: `HashMap<String, Page>` with layout tree + widget state per page
- Implement active page tracker
- Implement merge function: three-layer shallow merge with one-level-deep nested merge for style objects
- Implement style dictionary: resolve string references to style objects

### Phase 2 — Layout Engine

- Parse recursive layout tree from JSON
- Convert `size` strings to Ratatui `Constraint` values
- Implement nested container splitting (horizontal/vertical)
- Apply `margin` to layout nodes
- Handle `visible: false` widgets (reserve space, render blank)

### Phase 3 — Widget Renderers

- Paragraph (with inline styled text / span array support)
- Gauge
- LineGauge
- List (with optional scrollbar)
- Table (with optional scrollbar, column width constraints)
- Tabs
- Sparkline
- BarChart
- Border and title decoration on all widgets

### Phase 4 — Patch and Navigation

- Parse `patch` messages with page targeting
- Shallow merge into existing widget state (including nested style)
- Visibility toggle via patch
- Parse and handle `navigate` messages
- Background patch support (updating non-active pages)

### Phase 5 — Focus and Event System

- Build per-page focus ring from layout tree (depth-first, focusable widgets only)
- Implement Tab / Shift+Tab focus cycling
- Preserve focus position across page navigations
- Apply visual focus indicator to focused widget
- Capture keyboard events via Crossterm
- Map interactions to JSON event messages (select, submit, change, key)
- Send events to backend via transport trait (first implementation: stdout)

### Phase 6 — Hardening

- Implement all error handling per Section 10
- Full color system: named colors, `#RRGGBB` hex, `color(N)` 256-palette
- stderr logging for all ignored/invalid input
- Graceful terminal restore on panic or exit

### Phase 7 — Testing and Documentation

- Example JSON files for each widget type
- Unit tests for merge logic, layout constraint parsing, color parsing, style resolution
- Multi-page scenario integration test
- Example backend application (simple dashboard with multiple pages, periodic patches)
- README with quick-start guide and full schema reference

---

## 12. Minimal Example

The smallest possible valid `render` message:

```json
{
  "msg": "render",
  "page_order": ["main"],
  "pages": {
    "main": {
      "layout": { "ref": "hello" },
      "widgets": [
        { "id": "hello", "type": "paragraph", "text": "Hello, world!" }
      ]
    }
  },
  "active": "main"
}
```

This renders a single paragraph filling the entire terminal. All other properties come from defaults.

---

## 13. Design Decisions Log

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Transport layer | Trait-abstracted | Frontend doesn't care how JSON arrives |
| State management | Frontend holds all page states in memory | Instant page switching, no round-trip |
| Merge strategy | Three-layer shallow merge | Simple, predictable, minimal bandwidth |
| Style reuse | Named style dictionary in render message | Reduces JSON size, enables theming |
| Layout model | Recursive tree with constraint strings | Maps directly to Ratatui's Layout API |
| Focus management | Frontend-internal, depth-first ring | Ratatui has no focus system; backend doesn't need focus events |
| Scroll management | Frontend-internal | Backend doesn't need scroll position |
| Event scope | Result-only (select, submit, change, key) | Backend only needs actionable outcomes |
| Error handling | Silent ignore + stderr log | Never crash, never block backend |
| Widget catalog | Ratatui native widgets only | No custom widgets in v1; extend later if needed |
| Removed: Canvas | Too verbose for JSON-driven use | Add later if needed |
| Removed: Chart (line/scatter) | Too many parameters, Sparkline/BarChart cover basics | Add later if needed |
| Removed: Mouse events | Rare in terminal/embedded context | Add later if needed |
| Page re-render | Full replace only (all pages) | No per-page partial re-render; keeps protocol simple |
