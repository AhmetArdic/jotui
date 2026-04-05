#!/usr/bin/env python3
"""
Demo backend for Jotui.
Sends a full render message, then periodic patches to simulate live data.

Usage:
    python demo.py | cargo run
"""

import json
import sys
import time
import math
import random

def send(msg):
    print(json.dumps(msg), flush=True)

def main():
    # Initial render with all widget types across 2 pages
    render = {
        "msg": "render",
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

    send(render)

    # Periodic patches to simulate live data
    tick = 0
    cpu_history = [10, 20, 30, 25, 40, 35, 50, 45, 30, 20, 15, 25, 35, 45, 55, 40, 30, 20]

    while True:
        time.sleep(0.5)
        tick += 1

        # Simulate CPU fluctuation
        cpu = int(35 + 30 * math.sin(tick * 0.3) + random.randint(-5, 5))
        cpu = max(0, min(100, cpu))

        # Simulate memory slowly climbing
        mem = min(100, 62 + tick // 10)

        # Disk I/O oscillating
        disk = int(45 + 20 * math.cos(tick * 0.2))

        # Update sparkline history
        cpu_history.append(cpu)
        if len(cpu_history) > 30:
            cpu_history = cpu_history[-30:]

        # Service loads
        services = [
            ["web", max(0, min(100, 82 + random.randint(-10, 10)))],
            ["api", max(0, min(100, 64 + random.randint(-8, 8)))],
            ["db", max(0, min(100, 45 + random.randint(-5, 5)))],
            ["cache", max(0, min(100, 28 + random.randint(-3, 3)))],
            ["queue", max(0, min(100, 51 + random.randint(-7, 7)))]
        ]

        # Color based on CPU level
        if cpu > 80:
            cpu_style = "danger"
        elif cpu > 60:
            cpu_style = "warning"
        else:
            cpu_style = {"fg": "green"}

        # Dashboard patches
        patch = {
            "msg": "patch",
            "page": "dashboard",
            "updates": [
                {"id": "cpu_gauge", "value": cpu, "style": cpu_style},
                {"id": "mem_gauge", "value": mem},
                {"id": "disk_line", "value": disk},
                {"id": "cpu_spark", "data": cpu_history},
                {"id": "bar_chart", "bars": services}
            ]
        }
        send(patch)

        # Add a log entry every 5 ticks
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
            send({
                "msg": "patch",
                "page": "dashboard",
                "updates": [{"id": "log_list", "items": new_logs}]
            })

        # Update process table CPU values on details page (background update)
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
            send({
                "msg": "patch",
                "page": "details",
                "updates": [{"id": "proc_table", "rows": rows}]
            })

        # Read events from frontend (non-blocking would be ideal, but for demo this is fine)
        # In a real backend, you'd read stdout of the frontend process

if __name__ == "__main__":
    try:
        main()
    except (BrokenPipeError, KeyboardInterrupt):
        pass
