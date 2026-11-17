"""Read LLVM's physical source-line counters; grouped LF/LH totals are metadata."""

from pathlib import Path


NUMERIC_FIELDS = {"FNF", "FNH", "BRF", "BRH", "LF", "LH"}


def natural(value):
    if not value.isascii() or not value.isdecimal():
        raise ValueError(f"invalid LCOV integer: {value!r}")
    return int(value)


def named_counter(value, line_number):
    number, separator, name = value.partition(",")
    if not separator or not name.strip():
        raise ValueError("malformed LCOV function record")
    count = natural(number)
    if line_number and count == 0:
        raise ValueError("LCOV source line must be positive")


def branch_counter(value):
    fields = value.split(",")
    if len(fields) != 4:
        raise ValueError("malformed LCOV branch record")
    numbers = [natural(field) for field in fields[:3]]
    if numbers[0] == 0:
        raise ValueError("LCOV source line must be positive")
    if fields[3] != "-":
        natural(fields[3])


def metadata(key, value):
    if key in NUMERIC_FIELDS:
        natural(value)
        return
    if key in ("FN", "FNDA"):
        named_counter(value, key == "FN")
        return
    if key == "BRDA":
        branch_counter(value)
        return
    raise ValueError(f"unknown LCOV field: {key}")


def add_line(value, counters):
    fields = value.split(",")
    if len(fields) != 2:
        raise ValueError("malformed LCOV DA record")
    number, count = (natural(field) for field in fields)
    if number == 0:
        raise ValueError("LCOV source line must be positive")
    if number in counters:
        raise ValueError(f"duplicate LCOV source line: {number}")
    counters[number] = count


def record_row(record):
    name, counters = None, {}
    for line in record:
        key, separator, value = line.partition(":")
        if not separator:
            raise ValueError("malformed LCOV record")
        if key == "TN":
            continue
        if key == "SF":
            if name is not None or not value.strip():
                raise ValueError("invalid or repeated LCOV source filename")
            name = value
            continue
        if name is None:
            raise ValueError("LCOV field appears before its source filename")
        if key == "DA":
            add_line(value, counters)
        else:
            metadata(key, value)
    return source_row(name, counters)


def source_row(name, counters):
    if name is None or not counters:
        raise ValueError("LCOV source has no line records")
    missing = sorted(number for number, count in counters.items() if count == 0)
    return name, len(counters) - len(missing), len(counters), missing


def records(text):
    record = []
    for line in text.splitlines():
        if not line:
            continue
        if line != "end_of_record":
            record.append(line)
            continue
        if not record:
            raise ValueError("empty LCOV record")
        yield record
        record = []
    if record:
        raise ValueError("unterminated LCOV record")


def rows(path):
    try:
        text = Path(path).read_text(encoding="utf-8")
    except (OSError, UnicodeError) as problem:
        raise ValueError(f"cannot read LCOV report: {problem}") from problem
    result, seen = [], set()
    for record in records(text):
        row = record_row(record)
        if row[0] in seen:
            raise ValueError(f"duplicate LCOV source filename: {row[0]}")
        seen.add(row[0])
        result.append(row)
    if not result:
        raise ValueError("LCOV report has no source files")
    return result
