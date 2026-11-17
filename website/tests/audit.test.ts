import { expect, test } from "bun:test";
import { auditArguments } from "../scripts/audit";

test("the temporary audit exception applies only to the reviewed advisory and retains the full report", () => {
  expect(auditArguments(new Date("2026-11-17T12:00:00Z"))).toEqual([
    "audit", "--json", "--ignore", "GHSA-fc5q-39xc-vjpf",
  ]);
  expect(auditArguments(new Date("2026-11-30T23:59:59Z"))).toEqual([
    "audit", "--json", "--ignore", "GHSA-fc5q-39xc-vjpf",
  ]);
});

test("the exception expires at its UTC boundary and invalid dates cannot suppress advisories", () => {
  for (const now of [new Date("2026-12-01T00:00:00Z"), new Date("2027-01-01T00:00:00Z"), new Date(NaN)]) {
    expect(auditArguments(now)).toEqual(["audit", "--json"]);
  }
});
