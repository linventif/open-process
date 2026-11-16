# DMN decision tables

The DMN editor provides a single decision table with typed inputs and outputs,
rules, hit policies and evaluation against JSON input. It uses a limited subset
of FEEL unary tests rather than a complete FEEL interpreter.

## Create a decision table

1. Choose **DMN decision table** from the new-document picker or open its example.
2. Add named input and output columns. Choose `string`, `number` or `boolean`
   for each column.
3. Add rules and fill the input tests and output literals. Rule annotations can
   explain why a result exists.
4. Choose the hit policy appropriate to the table.
5. Supply JSON input values using the input names, run evaluation and inspect
   matching rules and output values.
6. Resolve diagnostics, then save as `.opx`.

For a numeric input named `age`, tests such as `< 18` and `>= 18` separate
two ranges. String output literals such as `"minor"` and `"adult"` label the
results. Example evaluation input is `{"age": 21}`.

## Tests, outputs and hit policies

Supported input tests include a wildcard `-`, literals, numeric comparisons,
intervals, comma-separated alternatives and `not(...)`. Outputs are string,
number, boolean or `null` literals, rather than arbitrary expressions.

| Policy | Meaning |
| --- | --- |
| `UNIQUE` | At most one rule may match. Overlap is an evaluation error. |
| `FIRST` | The first matching rule supplies the result. Rule order matters. |
| `ANY` | Multiple matching rules must agree on the output. |
| `COLLECT` | Collect the matching results without an aggregation operator. |

Keep input JSON names and types consistent with the columns. A saved draft may
contain invalid names or expressions, but diagnostics block evaluation until
they are corrected.

## Import and export

**File → Open** imports a supported single-table DMN XML document. Multiple
decisions, dependency graphs, aggregations and unsupported structural constructs
are rejected. **File → Export** offers DMN XML and CSV. Save `.opx` to retain
the editable document in Open Process.

The table scrolls rather than using a graphical canvas camera. Inspector controls
and common history still apply. See the [shared interface](interface.md) and
[format catalogue](export-formats.md).

## Limits

Functions, contexts, dates, arithmetic, DRD, BKM and decision services are outside
the implemented subset. Numeric comparisons use `f64`, not full FEEL decimal
precision. Limits are 128 inputs, 128 outputs and 10,000 rules, with test
expressions limited to 4,096 bytes and 32 nesting levels. This feature does not
claim complete DMN conformance. See [detailed coverage](notations-ajoutees.md).
