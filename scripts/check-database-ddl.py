#!/usr/bin/env python3
"""Execute generator output against SQLite and check metadata and constraints.

First run: cargo run -p op-core --example database-ddl -- /tmp/open-process-ddl
Then run:  python3 scripts/check-database-ddl.py /tmp/open-process-ddl
"""
import json
from pathlib import Path
import sqlite3
import sys


def quoted(value):
    return '"' + value.replace('"', '""') + '"'


def rejects(connection, sql, parameters=()):
    try:
        connection.execute(sql, parameters)
    except sqlite3.IntegrityError:
        return
    raise AssertionError(f"Expected an integrity error for {sql}")


def verify(directory):
    cases = sorted(directory.glob("*.sqlite.sql"))
    assert cases, f"No generated SQLite scripts in {directory}"
    for path in cases:
        schema = json.loads(path.with_name(path.name.replace(".sqlite.sql", ".json")).read_text())
        connection = sqlite3.connect(":memory:")
        connection.executescript(path.read_text())
        assert connection.execute("PRAGMA foreign_keys").fetchone()[0] == 1
        for table in schema["tables"]:
            info = connection.execute(f"PRAGMA table_info({quoted(table['name'])})").fetchall()
            assert [row[1] for row in info] == [c["name"] for c in table["columns"]]
            for actual, column in zip(info, table["columns"]):
                assert bool(actual[3]) == (not column["nullable"] or column["primary_key"])
                assert bool(actual[5]) == column["primary_key"]
            foreign_keys = connection.execute(f"PRAGMA foreign_key_list({quoted(table['name'])})").fetchall()
            expected = sum(len(f["columns"]) for f in schema["foreign_keys"] if f["table"] == table["id"])
            assert len(foreign_keys) == expected
        assert connection.execute("PRAGMA foreign_key_check").fetchall() == []
        if path.name == "shop.sqlite.sql":
            connection.execute("INSERT INTO customers (email) VALUES (?)", ("alice@example.test",))
            connection.execute("INSERT INTO orders (customers_id) VALUES (1)")
            connection.execute("INSERT INTO order_items (orders_id) VALUES (1)")
            assert connection.execute("SELECT total FROM orders").fetchone()[0] == 0
            assert connection.execute("SELECT quantity FROM order_items").fetchone()[0] == 1
            rejects(connection, "INSERT INTO orders (customers_id) VALUES (999)")
            rejects(connection, "INSERT INTO customers (email) VALUES (?)", ("alice@example.test",))
            rejects(connection, "INSERT INTO customers DEFAULT VALUES")
            connection.execute("DELETE FROM orders WHERE id=1")
            assert connection.execute("SELECT COUNT(*) FROM order_items").fetchone()[0] == 0
        if path.name == "literal-defaults.sqlite.sql":
            table = quoted(schema["tables"][0]["name"])
            connection.execute(f"INSERT INTO {table} DEFAULT VALUES")
            values = connection.execute(f"SELECT quoted, quantity, active FROM {table}").fetchone()
            assert values == ("O'Brien\\path'); DROP TABLE anything;--", 7, 1)
        connection.close()
        print(f"OK: {path.name}")


if __name__ == "__main__":
    verify(Path(sys.argv[1]))
