// Serial native execution, certificate readback, and Vox execution of the same
// compiled retained-update leaf. Input files and raw outputs remain reproducible.
import fs from 'node:fs';
import path from 'node:path';
import cp from 'node:child_process';
import crypto from 'node:crypto';
import {fileURLToPath} from 'node:url';
const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(__dirname, '../..');
const root = path.dirname(repo);
const wide = process.argv.includes('--wide');
const next = process.argv.includes('--next');
function option(name) {
  const index = process.argv.indexOf(name);
  if (index < 0) return undefined;
  const value = process.argv[index + 1];
  if (!value || value.startsWith('--')) throw Error(name + ' requires a value');
  return value;
}
const sourceFile = option('--sources');
const userSourceFile = option('--source-file');
const outputName = option('--output');
if (sourceFile && userSourceFile) throw Error('select one source input');
if ((sourceFile || userSourceFile) && !outputName) throw Error('explicit sources require a fresh --output directory');
const output = path.join(__dirname, outputName ?? (next ? 'stress_next' : wide ? 'stress_wide' : 'stress'));
if ((sourceFile || userSourceFile) && fs.existsSync(output)) throw Error('preserve existing measurement directory: ' + output);
fs.mkdirSync(output, {recursive: true});
const write = (file, value) => fs.writeFileSync(file, JSON.stringify(value, null, 2) + '\n');
const cpu = fs.readFileSync('/proc/self/status', 'utf8').match(/^Cpus_allowed_list:\s*(\d+)/m)[1];
function execute(name, command, args, input, limit = 20) {
  const stem = path.join(output, name);
  if (input !== undefined) write(stem + '.input.json', input);
  const descriptors = [fs.openSync(input === undefined ? '/dev/null' : stem + '.input.json', 'r'),
    fs.openSync(stem + '.stdout', 'w'), fs.openSync(stem + '.stderr', 'w')];
  const result = cp.spawnSync('/usr/bin/time', ['-f', '{"elapsed":%e,"rss_kib":%M,"user":%U,"system":%S,"exit":%x}',
    '-o', stem + '.time', 'timeout', '-k', '2s', limit + 's', 'taskset', '-c', cpu,
    'prlimit', '--as=2147483648', '--', command, ...args], {cwd: repo, stdio: descriptors});
  descriptors.forEach(fd => fs.closeSync(fd));
  const lines = fs.readFileSync(stem + '.time', 'utf8').trim().split('\n');
  const metrics = JSON.parse(lines[lines.length - 1]);
  return {status: result.status, metrics, stdout: fs.readFileSync(stem + '.stdout', 'utf8'),
    stderr: fs.readFileSync(stem + '.stderr', 'utf8'), record: path.relative(repo, stem)};
}
const clocksFile = path.join(__dirname, 'certified_clocks.json');
const clocks = JSON.parse(fs.readFileSync(clocksFile, 'utf8'));
if (clocks.schema !== 'retained-runtime-clock-v1' || clocks.clocks.length !== 256) throw Error('incomplete exported clocks');
const template = JSON.parse(fs.readFileSync(path.join(__dirname, 'coherent_fourier_program.json'), 'utf8'));
const historicalSources = ['increasing_20261004', 'wider_20261005'].flatMap(folder =>
  fs.readFileSync(path.join(repo, 'measurements/ququart', folder, 'sources.tsv'), 'utf8')
    .trim().split('\n').slice(1).map(line => {
      const fields = line.split('\t');
      return {bits: Number(fields[1]), source: fields[2], provenance: folder};
    })).sort((a, b) => a.bits - b.bits);
const sources = userSourceFile ? [JSON.parse(fs.readFileSync(path.join(__dirname, userSourceFile), 'utf8'))] : sourceFile || wide || next ? fs.readFileSync(path.join(__dirname, sourceFile ?? (next ? 'balanced_next.tsv' : 'balanced_wide.tsv')), 'utf8').trim().split('\n').slice(1)
  .map(line => {
    const [caseIndex, bits, source, p, q] = line.split('\t');
    const pBits = BigInt(p).toString(2).length, qBits = BigInt(q).toString(2).length;
    if (Math.abs(pBits - qBits) > 1 || BigInt(p) * BigInt(q) !== BigInt(source)) throw Error('source balance changed');
    return {bits: Number(bits), source, p_bits: pBits, q_bits: qBits, provenance: sourceFile ? path.basename(sourceFile, '.tsv') : next ? 'balanced_next' : 'balanced_wide'};
  }) : historicalSources;
const reports = [];
if (process.argv.includes('--controls')) {
  const baseline = JSON.parse(fs.readFileSync(path.join(__dirname, 'stress/increasing_20261004_128_r0_d0.input.json'), 'utf8'));
  const results = [];
  for (const corruption of ['table', 'seed', 'period', 'retained']) {
    const plan = structuredClone(baseline);
    const action = plan.steps[7].actions.find(a => a.kind === 'clock');
    if (corruption === 'table') action.certificate.table[1] ^= 1;
    if (corruption === 'seed') action.certificate.ticks[0] ^= 1;
    if (corruption === 'period') action.certificate.period = 1;
    if (corruption === 'retained') action.certificate = clocks.clocks.find(c => c.retained === 9 && c.seed === 1);
    const result = execute('control_' + corruption, 'target/debug/sic-tool', ['anyon-program'], plan);
    if (result.status !== 2 || !result.stderr.includes('clock')) throw Error('invalid clock admitted: ' + corruption);
    results.push({corruption, status: result.status, error: result.stderr.trim(), metrics: result.metrics});
  }
  write(path.join(output, 'clock_controls.json'), results);
  process.exit(0);
}
for (const source of sources) {
  if (BigInt(source.source).toString(2).length !== source.bits) throw Error('source width changed');
  for (const retained of [0, 9]) for (const digit of [0, 1, 2, 3]) {
    const certificate = clocks.clocks.find(c => c.retained === retained && c.seed === 1);
    const plan = structuredClone(template);
    plan.source = source.source;
    plan.digit = digit;
    const observation = axis => ({kind: 'evidence', proposition: 'measured-frame', axis,
      witness: 'retained-SIC-mass', outcome: axis === 'support' ? 0 : 1,
      numerator: retained ? '0' : '1', denominator: '1'});
    plan.steps[7].actions = [observation('support'), observation('refutation'),
      {kind: 'clock', id: 'frame-clock', proposition: 'measured-frame', certificate}];
    plan.steps[8].actions.unshift(...Array.from({length: 16}, () => ({kind: 'tick', id: 'frame-clock'})));
    plan.steps[10].actions.push({kind: 'clock_read', id: 'frame-clock', tick: 1000000000000000000});
    const name = `${source.provenance}_${source.bits}_r${retained}_d${digit}`;
    const native = execute(name, 'target/debug/sic-tool', ['anyon-program'], plan);
    if (native.status !== 0) {
      const stop = {source, digit, retained, status: native.status, metrics: native.metrics,
        stderr: native.stderr, record: native.record, operation: 'source-bound control reconstruction'};
      write(path.join(output, 'first_uncompleted.json'), stop);
      throw Error(name + ': stopped with status ' + native.status + ': ' + native.stderr);
    }
    const result = JSON.parse(native.stdout);
    const frame = result.events.find(e => e.kind === 'rejoin');
    const ticks = result.events.filter(e => e.kind === 'tick');
    if (ticks.length !== 16 || ticks.some(e => e.after !== e.cached)) throw Error('runtime clock mismatch');
    if (BigInt(frame.reconstruction_residual) > BigInt(frame.reconstruction_tolerance)) throw Error('reconstruction bound exceeded');
    const readback = execute(name + '_readback', 'target/debug/sic-tool', ['gram-reconstruct'],
      {source: source.source, gram: frame.gram, masses: frame.masses});
    if (readback.status !== 0) throw Error('certificate readback: ' + readback.stderr);
    const gram = frame.gram.map(([re, im]) => [re, (-BigInt(im)).toString()]);
    const control = execute(name + '_conjugate', 'target/debug/sic-tool', ['gram-reconstruct'],
      {source: source.source, gram, masses: frame.masses});
    if (control.status !== 2 || !control.stderr.includes('SIC masses differ')) throw Error('conjugate control was not rejected');
    const row = {source, digit, retained, fixed_point_bits: result.fixed_point_bits, ticks: ticks.length,
      entry: certificate.entry, period: certificate.period, residual: frame.reconstruction_residual,
      tolerance: frame.reconstruction_tolerance, return: result.source_return,
      native: {metrics: native.metrics, record: native.record},
      readback: {status: readback.status, metrics: readback.metrics}, control: {status: control.status, metrics: control.metrics}};
    reports.push(row);
    write(path.join(output, 'reconstruction_widths.json'), reports);
    console.log(JSON.stringify({bits: source.bits, digit, retained, elapsed: native.metrics.elapsed,
      rss_kib: native.metrics.rss_kib, period: certificate.period,
      residual_bits: BigInt(row.residual).toString(2).length,
      tolerance_bits: BigInt(row.tolerance).toString(2).length}));
  }
}
// The width sweep uses the same shared leaf already executed across all states.
if (!wide && !next && !sourceFile && !userSourceFile) {
// Execute a saved Vox IMASM module emitted from the compiled shared leaf.
const vox = path.join(root, 'Vox/target/release/vox');
const leaf = path.join(output, 'ququart_clock.so');
const module = execute('clock_module', vox, ['imasm', leaf]);
if (module.status !== 0) throw Error(module.stderr);
fs.writeFileSync(path.join(output, 'ququart_clock.imasm'), module.stdout);
const readings = [];
for (let retained = 0; retained < 16; retained++) for (let state = 0; state < 16; state++) {
  const certificate = clocks.clocks.find(c => c.retained === retained && c.seed === state);
  const packed = state | (retained << 4);
  const result = execute('vox_' + packed, vox, ['run', 'ququart_reentry_step',
    '--args', String(packed), path.join(output, 'ququart_clock.imasm')]);
  const match = result.stdout.match(/ = (\d+)\s+\[(\d+) steps/);
  if (result.status !== 0 || !match || Number(match[1]) !== certificate.table[state]) {
    throw Error('Vox compiled update mismatch: ' + packed + ': ' + result.stdout + result.stderr);
  }
  readings.push({packed, expected: certificate.table[state], observed: Number(match[1]), steps: Number(match[2])});
}
write(path.join(output, 'vox_clock_readings.json'), {mismatches: 0, readings,
  module_sha256: crypto.createHash('sha256').update(module.stdout).digest('hex'),
  clocks_sha256: crypto.createHash('sha256').update(fs.readFileSync(clocksFile)).digest('hex')});
console.log('Vox executed all 256 compiled retained updates with zero mismatches.');
}
