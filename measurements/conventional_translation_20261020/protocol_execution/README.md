# Native SRFP/DNN protocol execution check

`result.json` binds the final native executable and changed source files to
read rejection checks, the full native graph numerical control, and two
bounded full-family preparation observations. `control_200.json` contains all
12 executed nodes of each graph, native carrier flow agreement, exact forward
residual 2 for internally derived E=N-1, parity involution, shared bank
roundtrip and corrupted-root rejection. This negative control does not provide
factors. The source is a newly generated random balanced 200-bit semiprime;
reference factors are confined to `generation_reference.json` and are never
read by either native instrument or production preparation.

From G-mOMonadOS:

```sh
cargo build --offline --release --target x86_64-unknown-linux-gnu --target-dir target/conventional_translation_20261020 --lib --bin g-momonados -j1
rustc --edition=2021 -C opt-level=2 -C panic=abort --target x86_64-unknown-linux-gnu -L dependency=target/conventional_translation_20261020/x86_64-unknown-linux-gnu/release/deps --extern g_momonados=target/conventional_translation_20261020/x86_64-unknown-linux-gnu/release/libg_momonados.rlib measurements/conventional_translation_20261020/protocol_probe.rs -o measurements/conventional_translation_20261020/protocol_probe
timeout -s INT -k 5s 300s measurements/conventional_translation_20261020/protocol_probe measurements/conventional_translation_20261020/protocol_execution/fresh_balanced_200.txt > measurements/conventional_translation_20261020/protocol_execution/control_200.json
python3 measurements/conventional_translation_20261020/verify_protocol_execution.py
```

The final command runs two sequential externally bounded 20-second preparation
observations with an 8-GiB address-space limit. The fresh200 source selects SRFP;
the exact user896 source selects DNN. Both execute VINIT, IMSCRIB and enter AFWD.
Both reach the external boundary during AFWD without a prepared artifact.
These results locate the remaining growth inside the arithmetic operation.
They do not show flat preparation or completion of the complete tuple semantics.
No alternate producer, supplied-factor production input or small control was used.
