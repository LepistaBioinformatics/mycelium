"""Read gateway-delivered mail from the private test Mailpit API."""
import json
import os
import sys
from urllib.request import urlopen
from extract_magic_link import extract_display_url


def fetch(path):
    with urlopen(os.environ["PSIG_MAIL_API"] + path, timeout=5) as response:
        return json.load(response)


if __name__ == "__main__":
    origin, email = sys.argv[1:]
    result = ""
    for message in fetch("/api/v1/messages")["messages"]:
        if not any(recipient["Address"] == email for recipient in message["To"]):
            continue
        body = fetch("/api/v1/message/" + message["ID"])
        result = extract_display_url(body["HTML"] + "\n" + body["Text"], origin)
        if result:
            break
    print(result)
