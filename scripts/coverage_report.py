"""Validate native coverage reports and require every measured line to be covered."""

import json
from pathlib import Path
import sys

import coverage_lcov

def read_report(path):
    try:
        with Path(path).open(encoding="utf-8") as source:
            report = json.load(source)
    except (OSError, ValueError) as problem:
        raise ValueError(f"cannot read coverage report: {problem}") from problem
    if not isinstance(report, dict):
        raise ValueError("coverage report must be an object")
    return report


def counts(summary, fields):
    if not isinstance(summary, dict):
        raise ValueError("missing line counts")
    covered, count = (summary.get(field) for field in fields)
    if type(covered) is not int or type(count) is not int:
        raise ValueError("line counts must be exact integers")
    if not 0 <= covered <= count:
        raise ValueError(f"invalid line counts: {covered}/{count}")
    return covered, count


def filename(name):
    if not isinstance(name, str) or not name.strip():
        raise ValueError("missing source filename")
    return name


def verdict(label, totals, rows):
    """Check file counts as well as totals; never trust rounded percentages."""
    covered, count = totals
    if count == 0:
        raise ValueError("coverage measured no lines")
    for name, file_covered, file_count, missing in rows:
        if file_covered != file_count:
            suffix = f"; missing lines: {', '.join(map(str, missing))}" if missing else ""
            print(f"✗ {label}: {name}: {file_covered}/{file_count} lines{suffix}", file=sys.stderr)
    measured = (sum(row[1] for row in rows), sum(row[2] for row in rows))
    if totals != measured:
        raise ValueError(f"inconsistent totals {covered}/{count}; file counts {measured[0]}/{measured[1]}")
    passed = covered == count
    hundredths = covered * 10000 // count
    share = f"{hundredths // 100}.{hundredths % 100:02d}%"
    mark = "✓" if passed else "✗"
    message = f"{mark} {label}: {share} line coverage ({covered}/{count}); 100% required"
    print(message, file=sys.stdout if passed else sys.stderr)
    return passed


MIN_RUST_REGION_BASIS_POINTS = 9500


def rust_regions(path):
    companion = Path(path).with_suffix(".json")
    if not companion.is_file():
        return None
    data = read_report(companion).get("data")
    if not isinstance(data, list) or not data or not isinstance(data[0], dict):
        raise ValueError("invalid Rust JSON coverage data")
    totals = data[0].get("totals")
    if not isinstance(totals, dict) or "regions" not in totals:
        return None
    return companion.name, counts(totals.get("regions"), ("covered", "count"))


def check_rust_regions(path):
    measured = rust_regions(path)
    if measured is None:
        return True
    name, (covered, count) = measured
    if count == 0:
        raise ValueError("Rust region report measured no regions")
    hundredths = covered * 10000 // count
    passed = hundredths >= MIN_RUST_REGION_BASIS_POINTS
    share = f"{hundredths // 100}.{hundredths % 100:02d}%"
    floor = f"{MIN_RUST_REGION_BASIS_POINTS // 100}.{MIN_RUST_REGION_BASIS_POINTS % 100:02d}%"
    mark = "✓" if passed else "✗"
    message = f"{mark} Rust regions ({name}): {share} region coverage ({covered}/{count}); >={floor} required"
    print(message, file=sys.stdout if passed else sys.stderr)
    return passed


def check_rust(path):
    rows = coverage_lcov.rows(path)
    totals = sum(row[1] for row in rows), sum(row[2] for row in rows)
    lines_ok = verdict(f"Rust ({Path(path).name})", totals, rows)
    regions_ok = check_rust_regions(path)
    return lines_ok and regions_ok


def missing_lines(entry, uncovered):
    if "missing_lines" not in entry:
        return []
    missing = entry["missing_lines"]
    if not isinstance(missing, list) or any(type(line) is not int or line < 1 for line in missing):
        raise ValueError("invalid Python missing lines")
    if len(set(missing)) != len(missing) or len(missing) != uncovered:
        raise ValueError("Python missing lines disagree with line counts")
    return sorted(missing)


def python_rows(files):
    if not isinstance(files, dict) or not files:
        raise ValueError("Python report has no files")
    rows = []
    for name, entry in files.items():
        if not isinstance(entry, dict):
            raise ValueError("invalid Python source entry")
        covered, count = counts(entry.get("summary"), ("covered_lines", "num_statements"))
        missing = missing_lines(entry, count - covered)
        rows.append((filename(name), covered, count, missing))
    return rows


def check_python(path):
    report = read_report(path)
    rows = python_rows(report.get("files"))
    totals = counts(report.get("totals"), ("covered_lines", "num_statements"))
    return verdict(f"Python ({Path(path).name})", totals, rows)
