#!/usr/bin/env python3
"""Measure REQ-NF-001 API latency against a freshly seeded local server."""
import concurrent.futures
import json
import math
import os
import statistics
import threading
import time
import urllib.error
import urllib.request

BASE = os.environ.get("FIXOPS_PERF_URL", "http://127.0.0.1:18080") + "/api/v1"
PASSWORD = "performance-test-password"
ROUNDS = 20


def request(method, path, token=None, body=None):
    data = None if body is None else json.dumps(body).encode()
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = "Bearer " + token
    req = urllib.request.Request(BASE + path, data=data, headers=headers, method=method)
    started = time.perf_counter()
    try:
        with urllib.request.urlopen(req, timeout=30) as response:
            payload = response.read()
            elapsed = time.perf_counter() - started
            return response.status, json.loads(payload), elapsed
    except urllib.error.HTTPError as error:
        error.read()
        return error.code, None, time.perf_counter() - started


def expect(method, path, status, token=None, body=None):
    actual, result, _ = request(method, path, token, body)
    if actual != status:
        raise RuntimeError(f"{method} {path}: expected {status}, got {actual}")
    return result


def main():
    admin = expect("POST", "/sessions", 201, body={"username": "perf.admin", "password": PASSWORD})["token"]
    staff = expect("GET", "/staff", 200, admin)
    technician_id = next(account["id"] for account in staff if account["username"] == "perf.technician")
    tokens = [admin]
    for n in range(1, 10):
        username = f"perf.staff{n}"
        expect("POST", "/staff", 201, admin, {"username": username, "password": PASSWORD, "role": "Administrator"})
        tokens.append(expect("POST", "/sessions", 201, body={"username": username, "password": PASSWORD})["token"])

    barrier = threading.Barrier(10)

    def worker(worker_id):
        token = tokens[worker_id]
        samples = []
        barrier.wait()
        for n in range(ROUNDS):
            record_id = worker_id * ROUNDS + n + 1
            operations = [
                ("client search", "GET", f"/clients?q=Performance%20Client%20{record_id}", None, 200),
                ("device search", "GET", f"/devices?q=PERF-SERIAL-{record_id}", None, 200),
                ("order open", "GET", f"/orders/{record_id}", None, 200),
                ("order save", "POST", "/orders", {"ticket_number": f"PERF-NEW-{record_id}", "client_id": record_id, "device_id": record_id, "assigned_technician_id": technician_id}, 201),
                ("allowed transition", "PUT", f"/orders/{record_id}/diagnosis", {"diagnostic_findings": "Performance test findings"}, 200),
            ]
            for label, method, path, body, expected in operations:
                status, result, elapsed = request(method, path, token, body)
                valid = status == expected
                if valid and label in ("client search", "device search"):
                    valid = any(item["id"] == record_id for item in result)
                if valid and label == "order open":
                    valid = result["id"] == record_id
                if valid and label == "allowed transition":
                    valid = result["status"] == "Diagnosing" and result["diagnostic_findings"] == body["diagnostic_findings"]
                samples.append((label, elapsed, valid, status))
        return samples

    with concurrent.futures.ThreadPoolExecutor(max_workers=10) as executor:
        samples = [sample for group in executor.map(worker, range(10)) for sample in group]
    failed = False
    for label in ("client search", "device search", "order open", "order save", "allowed transition"):
        group = [(elapsed, valid, status) for name, elapsed, valid, status in samples if name == label]
        times = sorted(elapsed for elapsed, _, _ in group)
        p95 = times[math.ceil(len(times) * .95) - 1]
        errors = [status for _, valid, status in group if not valid]
        failed |= bool(errors) or p95 > 2
        print(f"{label}: n={len(times)} p50={statistics.median(times)*1000:.1f}ms p95={p95*1000:.1f}ms max={max(times)*1000:.1f}ms errors={len(errors)} statuses={sorted(set(errors))}")
    print("API check:", "FAIL" if failed else "PASS", "(browser visible timing not measured)")
    return int(failed)


if __name__ == "__main__":
    raise SystemExit(main())
