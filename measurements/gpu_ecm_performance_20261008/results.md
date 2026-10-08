GPU ECM defaults to 256 concurrent Suyama Montgomery curves. Each curve uses a cooperative warp over IMASM parity arms. Carry propagation uses lane-control ballots. Coordinates stay in Montgomery form; stage one checks prime-power steps and uses doubling and tripling chains. Singular curves are discarded, or their non-unit discriminant provides a factor. The CUDA context and module stay loaded between requests.

Release-build factor extraction on the existing witnesses uses B1=100, B2=100, sigma starting at 6, and a 256-curve budget on both routes. Each result returns a proper factor and closes by exact IMASM division. CPU medians measure five complete process invocations. GPU medians measure the last five of seven route invocations in one process following the GPU arithmetic check. Those GPU timings include schedule preparation, transfers, execution, and factor closure, but exclude initial CUDA setup, PTX compilation, and process startup.

| Source bits | CPU median ms | Warm GPU median ms | CPU/GPU |
| --- | --- | --- | --- |
| 256 | 27.96 | 12.38 | 2.26 |
| 512 | 66.72 | 37.58 | 1.78 |
| 1048 | 186.59 | 70.10 | 2.66 |

A complete 32-curve stage-one batch on the balanced 256-bit input takes 3.243595 seconds on CPU and 0.420338 seconds on GPU, including process startup. Both finish the same sigma range with no factor. GPU process speedup is 7.72.
A complete 256-curve stage-one batch on the balanced 256-bit input takes 26.208492 seconds on CPU and 0.493685 seconds on GPU, including process startup. Both finish the same sigma range with no factor. GPU process speedup is 53.09.

One-curve extraction on the easy witnesses still favors CPU. The GPU advantage uses concurrent curves. First-time kernel compilation remains an additional cost. The batch and latency JSON files record the GPU source hash, release profile, parameters, and measurements. The benchmark scripts reproduce the comparison. The wiring diagrams describe the existing ECM stage and scalar operator words.
