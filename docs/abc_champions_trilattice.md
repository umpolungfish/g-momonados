# ABC champion events in the native trilattice

`abc champions` is the displacement view of the finite ABC measurement stream.
It scans admissible coprime triples in increasing `c` and emits an event only
when a new triple exceeds the current discrepancy reading.

Every event carries `B4::B` (`state: "B"` in JSON). The event preserves both
sides of the transition: the previous champion remains available as evidence
and the newly selected champion is available as the current reading. The
floating point discrepancy is an observable attached to that trilattice
register; it is not used as a proof object.

Examples:

```text
abc champions 0.1 9
abc champions --json 0.1 9
```

The JSON form includes `truth` and `falsehood` lanes so downstream tools can
consume the native `B` state without reconstructing it from the event shape.
