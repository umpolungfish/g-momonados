# Recursive containment width sweep

The hosted release selector received the exact close-factor family

```text
P(w) = 2^(w - 1) + 11
Q(w) = 2^(w - 1) + 57
N(w) = P(w) × Q(w)
```

The runner starts a fresh `--selector-relation` process for every sample and
accepts its duration only when the emitted witness is `N(w) = P(w) x Q(w)
(verified)`. The phase-relation release control passed its eleven tests before
the sweep.

| factor width | target width | fresh processes | median | minimum | mean |
|---:|---:|---:|---:|---:|---:|
| 128 | 255 | 5 | 2.3449 ms | 2.0184 ms | 2.3662 ms |
| 256 | 511 | 5 | 2.6510 ms | 2.5574 ms | 2.7371 ms |
| 512 | 1,023 | 5 | 4.9587 ms | 4.3418 ms | 5.2278 ms |
| 1,024 | 2,047 | 5 | 11.2821 ms | 11.1548 ms | 11.5527 ms |
| 2,048 | 4,095 | 5 | 36.6321 ms | 35.6894 ms | 36.7340 ms |
| 4,096 | 8,191 | 5 | 212.0372 ms | 206.9435 ms | 214.0321 ms |
| 8,192 | 16,383 | 5 | 1.0069 s | 991.1553 ms | 1.0065 s |
| 16,384 | 32,767 | 5 | 4.4845 s | 4.4019 s | 4.4848 s |
| 32,768 | 65,535 | 5 | 18.9126 s | 18.6904 s | 20.5201 s |
| 65,536 | 131,071 | 1 | 83.1491 s | 83.1491 s | 83.1491 s |

Every raw row carries the exact decimal `p`, `q`, and `target` used for that
measurement, alongside its durations. The durations through 32,768 bits are in
`recursive_containment_width_sweep.jsonl`. The 65,536-bit row is in
`recursive_containment_width_65536.jsonl`. Re-run the same family with:

```text
python3 measurements/recursive_containment_width_sweep.py
```
