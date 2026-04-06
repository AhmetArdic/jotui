#!/usr/bin/env python3
"""
Demo backend for Jotui.
Sends a full render message, then periodic patches to simulate live data.
Reads events from Jotui via TCP.

Usage:
    python examples/demo.py
"""

import json
import socket
import subprocess
import sys
import time
import math
import random
import threading
import os


def send(sock, method, params):
    """Send a JSON-RPC 2.0 notification with Content-Length framing over TCP."""
    body = json.dumps({"jsonrpc": "2.0", "method": method, "params": params})
    header = f"Content-Length: {len(body)}\r\n\r\n"
    sock.sendall(header.encode() + body.encode())


def read_message(sockfile):
    """Read a JSON-RPC 2.0 message with Content-Length framing from a socket file."""
    content_length = None
    while True:
        line = sockfile.readline()
        if not line:
            return None
        line = line.decode().strip()
        if not line:
            break
        if line.startswith("Content-Length:"):
            content_length = int(line.split(":", 1)[1].strip())

    if content_length is None:
        return None

    body = sockfile.read(content_length)
    if not body:
        return None
    return json.loads(body)


def event_reader(sockfile, log_file=None):
    """Read events from Jotui via TCP in a background thread."""
    while True:
        try:
            msg = read_message(sockfile)
            if msg is None:
                break
            if log_file:
                json.dump(msg, log_file)
                log_file.write("\n")
                log_file.flush()
        except Exception:
            break


def main():
    project_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
    cargo_target = os.path.join(project_dir, "target", "debug", "jotui")
    if os.name == "nt":
        cargo_target += ".exe"

    if os.path.isfile(cargo_target):
        cmd = [cargo_target]
    else:
        cmd = ["cargo", "run", "--quiet", "--"]

    # Backend owns the TCP listener
    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.bind(("127.0.0.1", 0))
    server.listen(1)
    port = server.getsockname()[1]

    # Spawn Jotui — no pipes, stdout/stdin/stderr go straight to terminal
    proc = subprocess.Popen(
        cmd + ["--port", str(port)],
        cwd=project_dir,
    )

    # Wait for Jotui to connect
    server.settimeout(10)
    try:
        conn, _addr = server.accept()
    except socket.timeout:
        print("Jotui did not connect in time", file=sys.stderr)
        proc.terminate()
        sys.exit(1)
    server.close()

    sockfile = conn.makefile("rb")

    # Start event reader thread (logs to file if --log is given)
    log_file = None
    if "--log" in sys.argv:
        idx = sys.argv.index("--log")
        log_path = sys.argv[idx + 1] if idx + 1 < len(sys.argv) else "events.log"
        log_file = open(log_path, "a")
    t = threading.Thread(target=event_reader, args=(sockfile, log_file), daemon=True)
    t.start()

    # Initial render with all widget types across 2 pages
    render_params = {
        "styles": {
            "danger": {"fg": "red", "bold": True},
            "ok": {"fg": "green", "bold": True},
            "warning": {"fg": "yellow", "bold": True},
            "header": {"fg": "cyan", "bold": True},
            "muted": {"fg": "dark_gray"}
        },
        "page_order": ["dashboard", "details"],
        "pages": {
            "dashboard": {
                "layout": {
                    "children": [
                        {"size": "1", "ref": "nav_tabs"},
                        {"size": "3", "ref": "title"},
                        {
                            "dir": "h",
                            "children": [
                                {
                                    "size": "70%",
                                    "children": [
                                        {"size": "3", "ref": "cpu_gauge"},
                                        {"size": "3", "ref": "mem_gauge"},
                                        {"size": "3", "ref": "disk_line"},
                                        {"ref": "log_list"}
                                    ]
                                },
                                {
                                    "children": [
                                        {"size": "8", "ref": "cpu_spark"},
                                        {"ref": "bar_chart"},
                                        {"size": "1", "ref": "status_bar"}
                                    ]
                                }
                            ]
                        }
                    ]
                },
                "widgets": [
                    {
                        "id": "nav_tabs",
                        "type": "tabs",
                        "titles": ["Dashboard", "Details"],
                        "selected": 0,
                        "focusable": True,
                        "highlight_style": {"fg": "cyan", "bold": True}
                    },
                    {
                        "id": "title",
                        "type": "paragraph",
                        "text": [
                            {"text": " System Monitor ", "fg": "cyan", "bold": True},
                            " — ",
                            {"text": "Jotui demo", "fg": "dark_gray"}
                        ],
                        "align": "center",
                        "border": "rounded",
                        "title": "Jotui"
                    },
                    {
                        "id": "cpu_gauge",
                        "type": "gauge",
                        "value": 35,
                        "max": 100,
                        "label": "CPU",
                        "border": "rounded",
                        "title": "CPU Usage",
                        "style": {"fg": "green"}
                    },
                    {
                        "id": "mem_gauge",
                        "type": "gauge",
                        "value": 62,
                        "max": 100,
                        "label": "Memory",
                        "border": "rounded",
                        "title": "Memory Usage",
                        "style": {"fg": "cyan"}
                    },
                    {
                        "id": "disk_line",
                        "type": "line_gauge",
                        "value": 45,
                        "max": 100,
                        "label": "Disk I/O",
                        "border": "rounded",
                        "title": "Disk",
                        "style": {"fg": "yellow"}
                    },
                    {
                        "id": "log_list",
                        "type": "list",
                        "items": [
                            "System boot complete",
                            "Network interface eth0 up",
                            "SSH service started",
                            "Firewall rules loaded",
                            "NTP synchronized",
                            "Monitoring agent ready",
                            "Database connection OK",
                            "API server listening :8080"
                        ],
                        "selected": 0,
                        "scrollbar": True,
                        "border": "rounded",
                        "title": "System Logs",
                        "focusable": True,
                        "highlight_symbol": "▶ ",
                        "highlight_style": {"fg": "black", "bg": "cyan", "bold": True}
                    },
                    {
                        "id": "cpu_spark",
                        "type": "sparkline",
                        "data": [10, 20, 30, 25, 40, 35, 50, 45, 30, 20, 15, 25, 35, 45, 55, 40, 30, 20],
                        "border": "rounded",
                        "title": "CPU History",
                        "style": {"fg": "green"}
                    },
                    {
                        "id": "bar_chart",
                        "type": "bar_chart",
                        "bars": [
                            ["web", 82],
                            ["api", 64],
                            ["db", 45],
                            ["cache", 28],
                            ["queue", 51]
                        ],
                        "bar_width": 5,
                        "border": "rounded",
                        "title": "Service Load (%)"
                    },
                    {
                        "id": "status_bar",
                        "type": "paragraph",
                        "text": [
                            {"text": " ONLINE ", "fg": "black", "bg": "green", "bold": True},
                            " ",
                            {"text": "Tab", "fg": "yellow", "bold": True},
                            ": focus  ",
                            {"text": "↑↓", "fg": "yellow", "bold": True},
                            ": select  ",
                            {"text": "←→", "fg": "yellow", "bold": True},
                            ": tabs  ",
                            {"text": "Enter", "fg": "yellow", "bold": True},
                            ": confirm  ",
                            {"text": "Ctrl+Q", "fg": "red", "bold": True},
                            ": quit"
                        ],
                        "style": "muted"
                    }
                ]
            },
            "details": {
                "layout": {
                    "children": [
                        {"size": "1", "ref": "nav_tabs2"},
                        {"size": "3", "ref": "detail_title"},
                        {"ref": "proc_table"},
                        {"size": "1", "ref": "status_bar2"}
                    ]
                },
                "widgets": [
                    {
                        "id": "nav_tabs2",
                        "type": "tabs",
                        "titles": ["Dashboard", "Details"],
                        "selected": 1,
                        "focusable": True,
                        "highlight_style": {"fg": "cyan", "bold": True}
                    },
                    {
                        "id": "detail_title",
                        "type": "paragraph",
                        "text": [
                            {"text": " Process Details ", "fg": "magenta", "bold": True}
                        ],
                        "align": "center",
                        "border": "double",
                        "title": "Details"
                    },
                    {
                        "id": "proc_table",
                        "type": "table",
                        "headers": ["PID", "Name", "CPU %", "Mem MB", "Status"],
                        "rows": [
                            ["1", "systemd", "0.1", "12", "running"],
                            ["245", "sshd", "0.0", "8", "running"],
                            ["512", "nginx", "2.3", "64", "running"],
                            ["789", "postgres", "5.1", "256", "running"],
                            ["1024", "node", "12.4", "512", "running"],
                            ["1337", "redis", "1.2", "128", "running"],
                            ["2048", "prometheus", "3.7", "384", "running"],
                            ["4096", "grafana", "2.1", "196", "running"]
                        ],
                        "widths": ["10%", "25%", "15%", "15%", "*"],
                        "selected": 0,
                        "scrollbar": True,
                        "border": "rounded",
                        "title": "Processes",
                        "focusable": True,
                        "highlight_style": {"fg": "black", "bg": "magenta", "bold": True},
                        "header_style": {"fg": "yellow", "bold": True}
                    },
                    {
                        "id": "status_bar2",
                        "type": "paragraph",
                        "text": [
                            {"text": " ONLINE ", "fg": "black", "bg": "green", "bold": True},
                            " ",
                            {"text": "Tab", "fg": "yellow", "bold": True},
                            ": focus  ",
                            {"text": "↑↓", "fg": "yellow", "bold": True},
                            ": select  ",
                            {"text": "←→", "fg": "yellow", "bold": True},
                            ": tabs  ",
                            {"text": "Ctrl+Q", "fg": "red", "bold": True},
                            ": quit"
                        ],
                        "style": "muted"
                    }
                ]
            }
        },
        "active": "dashboard"
    }

    send(conn, "render", render_params)

    # Periodic patches to simulate live data
    tick = 0
    cpu_history = [10, 20, 30, 25, 40, 35, 50, 45, 30, 20, 15, 25, 35, 45, 55, 40, 30, 20]

    try:
        while proc.poll() is None:
            time.sleep(0.5)
            tick += 1

            cpu = int(35 + 30 * math.sin(tick * 0.3) + random.randint(-5, 5))
            cpu = max(0, min(100, cpu))
            mem = min(100, 62 + tick // 10)
            disk = int(45 + 20 * math.cos(tick * 0.2))

            cpu_history.append(cpu)
            if len(cpu_history) > 30:
                cpu_history = cpu_history[-30:]

            services = [
                ["web", max(0, min(100, 82 + random.randint(-10, 10)))],
                ["api", max(0, min(100, 64 + random.randint(-8, 8)))],
                ["db", max(0, min(100, 45 + random.randint(-5, 5)))],
                ["cache", max(0, min(100, 28 + random.randint(-3, 3)))],
                ["queue", max(0, min(100, 51 + random.randint(-7, 7)))]
            ]

            if cpu > 80:
                cpu_style = "danger"
            elif cpu > 60:
                cpu_style = "warning"
            else:
                cpu_style = {"fg": "green"}

            send(conn, "patch", {
                "page": "dashboard",
                "updates": [
                    {"id": "cpu_gauge", "value": cpu, "style": cpu_style},
                    {"id": "mem_gauge", "value": mem},
                    {"id": "disk_line", "value": disk},
                    {"id": "cpu_spark", "data": cpu_history},
                    {"id": "bar_chart", "bars": services}
                ]
            })

            if tick % 5 == 0:
                new_logs = [
                    f"[{tick:04d}] CPU: {cpu}% | Mem: {mem}% | Disk: {disk}%",
                    "System boot complete",
                    "Network interface eth0 up",
                    "SSH service started",
                    "Firewall rules loaded",
                    "NTP synchronized",
                    "Monitoring agent ready",
                    "Database connection OK",
                    "API server listening :8080"
                ]
                send(conn, "patch", {
                    "page": "dashboard",
                    "updates": [{"id": "log_list", "items": new_logs}]
                })

            if tick % 3 == 0:
                rows = [
                    ["1", "systemd", f"{random.uniform(0, 0.5):.1f}", "12", "running"],
                    ["245", "sshd", f"{random.uniform(0, 0.3):.1f}", "8", "running"],
                    ["512", "nginx", f"{random.uniform(1, 5):.1f}", "64", "running"],
                    ["789", "postgres", f"{random.uniform(3, 8):.1f}", "256", "running"],
                    ["1024", "node", f"{random.uniform(8, 20):.1f}", "512", "running"],
                    ["1337", "redis", f"{random.uniform(0.5, 3):.1f}", "128", "running"],
                    ["2048", "prometheus", f"{random.uniform(2, 6):.1f}", "384", "running"],
                    ["4096", "grafana", f"{random.uniform(1, 4):.1f}", "196", "running"]
                ]
                send(conn, "patch", {
                    "page": "details",
                    "updates": [{"id": "proc_table", "rows": rows}]
                })
    except (BrokenPipeError, KeyboardInterrupt, OSError):
        pass
    finally:
        conn.close()
        if proc.poll() is None:
            proc.terminate()


if __name__ == "__main__":
    main()
