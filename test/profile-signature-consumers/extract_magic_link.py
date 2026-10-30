"""Parse test magic links/codes from stub logs or message HTML.

Pure helpers shared by smoke runners. Only loopback URLs are returned.
"""
import argparse
import html
import re
import sys
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import urlsplit

ANSI = re.compile(r"\x1b\[[0-9;]*m")
WRAPPED_LINE = re.compile(r"\r?\n +")
DISPLAY_URL = re.compile(r"""https?://[^\s\[\]<>"']+/magic-link/display\?[^\s\[\]<>"']+""")


def extract_display_url(text: str, expected_origin: str | None = None) -> str:
    clean = html.unescape(WRAPPED_LINE.sub("", ANSI.sub("", text)))
    for candidate in reversed(DISPLAY_URL.findall(clean)):
        candidate = candidate.rstrip(".,)")
        parsed = urlsplit(candidate)
        if parsed.hostname not in {"localhost", "127.0.0.1", "::1"}:
            continue
        if expected_origin is not None and f"{parsed.scheme}://{parsed.netloc}" != expected_origin:
            continue
        return candidate
    return ""


class _CodeParser(HTMLParser):
    def __init__(self):
        super().__init__()
        self.in_code = False
        self.invalid = False
        self.values: list[str] = []

    def handle_starttag(self, tag, attrs):
        if self.in_code and tag in {"script", "style"}:
            self.invalid = True
        if tag == "div" and "code" in dict(attrs).get("class", "").split():
            self.in_code = True

    def handle_endtag(self, tag):
        if tag == "div":
            self.in_code = False

    def handle_data(self, value):
        if self.in_code:
            self.values.append(value)


def extract_code(page: str) -> str:
    parser = _CodeParser()
    parser.feed(page)
    candidate = "".join(parser.values).strip()
    return candidate if not parser.invalid and re.fullmatch(r"[0-9]{6}", candidate) else ""


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("path", nargs="?")
    parser.add_argument("--code", action="store_true")
    parser.add_argument("--expected-origin")
    args = parser.parse_args()
    text = Path(args.path).read_text(encoding="utf-8", errors="replace") if args.path else sys.stdin.read()
    print(extract_code(text) if args.code else extract_display_url(text, args.expected_origin))


if __name__ == "__main__":
    main()
