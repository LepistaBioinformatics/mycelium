"""Prove STARTTLS, hostname verification, auth and real delivery before gateway E2E."""
import json
import os
import smtplib
import ssl
import time
from email.message import EmailMessage
from urllib.request import urlopen

context = ssl.create_default_context(cafile=os.environ["SSL_CERT_FILE"])
message = EmailMessage()
message["From"] = "probe@localhost"
message["To"] = "probe@localhost"
message["Subject"] = "PSIG infrastructure probe"
message.set_content("Synthetic STARTTLS probe, no external relay.")
with smtplib.SMTP("localhost", int(os.environ["PSIG_SMTP_PORT"]), timeout=5) as smtp:
    smtp.starttls(context=context)
    smtp.login(os.environ["PSIG_SMTP_USER"], os.environ["PSIG_SMTP_PASSWORD"])
    smtp.send_message(message)
for attempt in range(20):
    with urlopen(os.environ["PSIG_MAIL_API"] + "/api/v1/messages", timeout=5) as response:
        messages = json.load(response)
    if any(m["Subject"] == message["Subject"] for m in messages["messages"]):
        print("PASS verified STARTTLS, authentication and SMTP delivery retrieved through API")
        break
    time.sleep(0.1)
else:
    raise RuntimeError("SMTP delivery did not reach Mailpit")
