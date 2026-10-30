"""Benchmark-only DB fixture: 200 real guest licenses, no runtime bypass."""
import sqlite3
import sys
import uuid
from datetime import datetime, timezone


def statements(account: str, email: str):
    uuid.UUID(account)
    if email != "psig-e2e@localhost":
        raise ValueError("only the synthetic benchmark user is accepted")
    now = datetime.now(timezone.utc).replace(tzinfo=None).isoformat()
    yield "BEGIN;"
    for index in range(200):
        role = str(uuid.uuid5(uuid.NAMESPACE_URL, f"psig-benchmark-role-{index}"))
        guest = str(uuid.uuid5(uuid.NAMESPACE_URL, f"psig-benchmark-guest-{index}"))
        name = f"psig-benchmark-{index:03}"
        yield f"INSERT INTO guest_role(id,name,slug,permission,created) VALUES('{role}','{name}','{name}',1,'{now}');"
        yield f"INSERT INTO guest_user(id,email,guest_role_id,was_verified,created) VALUES('{guest}','{email}','{role}',TRUE,'{now}');"
        yield f"INSERT INTO guest_user_on_account(guest_user_id,account_id,created) VALUES('{guest}','{account}','{now}');"
    yield "COMMIT;"


if __name__ == "__main__":
    sql = "\n".join(statements(sys.argv[2], sys.argv[3]))
    if sys.argv[1] == "sql":
        print(sql)
    else:
        with sqlite3.connect(sys.argv[1], timeout=5) as connection:
            connection.executescript(sql)
