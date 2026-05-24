#!/usr/bin/env python3
from __future__ import annotations

import pathlib
import re
import sys


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: check_concept_schemas.py <repo-root-directory>", file=sys.stderr)
        return 2
    root = pathlib.Path(sys.argv[1])
    files = sorted(root.glob("*/schema/*.concept.schema"))
    if not files:
        print(f"no concept schemas found under {root}", file=sys.stderr)
        return 1

    failures: list[str] = []
    for path in files:
        failures.extend(validate(path))

    if failures:
        for failure in failures:
            print(failure, file=sys.stderr)
        return 1

    print(f"(ConceptSchemasValidated {len(files)})")
    return 0


def validate(path: pathlib.Path) -> list[str]:
    text = path.read_text()
    failures: list[str] = []
    if ";;" in text:
        failures.append(f"{path}: schema source must not contain comments")
    if '"' in text:
        failures.append(f"{path}: schema source must not use quote-delimited strings")
    sections = top_level_sections(text)
    if len(sections) != 6:
        failures.append(f"{path}: expected six top-level sections, found {len(sections)}")
        return failures

    expected = [("{", "}"), ("[", "]"), ("[", "]"), ("[", "]"), ("{", "}"), ("[", "]")]
    for index, (section, (open_delimiter, close_delimiter)) in enumerate(zip(sections, expected), 1):
        if not (section.startswith(open_delimiter) and section.endswith(close_delimiter)):
            failures.append(
                f"{path}: section {index} must be {open_delimiter}...{close_delimiter}"
            )

    for section_index in (1, 2, 3):
        failures.extend(validate_header(path, section_index + 1, sections[section_index]))

    if "(Version 0 1)" not in sections[5]:
        failures.append(f"{path}: features section must contain (Version 0 1)")
    return failures


def validate_header(path: pathlib.Path, section_number: int, section: str) -> list[str]:
    failures: list[str] = []
    inner = section[1:-1].strip()
    if not inner:
        return failures
    for root in re.finditer(r"\(([A-Z][A-Za-z0-9]*)\s+([^\)]*)\)", inner):
        payload = root.group(2).strip()
        if not (payload.startswith("[") and payload.endswith("]")):
            failures.append(
                f"{path}: header section {section_number} root {root.group(1)} must use [SubVariant...]"
            )
    return failures


def top_level_sections(text: str) -> list[str]:
    sections: list[str] = []
    stack: list[str] = []
    start: int | None = None
    pairs = {"{": "}", "[": "]", "(": ")"}
    closing = {value: key for key, value in pairs.items()}
    for index, character in enumerate(text):
        if character in pairs:
            if not stack:
                start = index
            stack.append(character)
        elif character in closing:
            if not stack or stack[-1] != closing[character]:
                raise ValueError(f"unbalanced delimiter at byte {index}")
            stack.pop()
            if not stack and start is not None:
                sections.append(text[start : index + 1].strip())
                start = None
        elif not stack and not character.isspace():
            raise ValueError(f"unexpected top-level token {character!r} at byte {index}")
    if stack:
        raise ValueError("unclosed delimiter")
    return sections


if __name__ == "__main__":
    raise SystemExit(main())
