# ABC measurement stream export

The REPL command

```text
abc stream --json [eps] <cutoff...>
```

emits one JSON object with an `epsilon` field and a `measurements` array. Each
measurement contains `cutoff`, an `{a,b,c}` attained triple, the floating-point
`discrepancy`, the three-coordinate IUTT `packet`, and boolean
`radical_calibrated`, `height_calibrated`, and `cofinal` fields.

For example:

```bash
./run_cmds.sh 'abc stream --json 0.1 9 32 70'
```

The JSON line is suitable for archival and for a certificate-generation step
that emits Lean `WindowEnclosure` declarations. The floating-point values are
measurements; Lean remains the authority for exact enclosure proofs.
