// Bake N/base/radix/clock into the existing compute device and run one phase
// digit. Source-generation factors are never passed to preparation or execution.
import fs from 'node:fs';
import path from 'node:path';
import cp from 'node:child_process';
import crypto from 'node:crypto';
import {fileURLToPath} from 'node:url';
const directory = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(directory, '../..');
const output = path.join(directory, 'work_stress');
fs.mkdirSync(output, {recursive: true});
const clocks = JSON.parse(fs.readFileSync(path.join(directory, 'certified_clocks.json'), 'utf8')).clocks;
const cpu = fs.readFileSync('/proc/self/status', 'utf8').match(/^Cpus_allowed_list:\s*(\d+)/m)[1];
function run(name, command, args, env = {}, deadline = 30) {
  const stem = path.join(output, name);
  const fds = [fs.openSync('/dev/null', 'r'), fs.openSync(stem + '.stdout', 'w'), fs.openSync(stem + '.stderr', 'w')];
  const result = cp.spawnSync('/usr/bin/time', ['-f', '{"elapsed":%e,"rss_kib":%M,"user":%U,"system":%S,"exit":%x}',
    '-o', stem + '.time', 'timeout', '-k', '2s', deadline + 's', 'taskset', '-c', cpu,
    'prlimit', '--as=2147483648', '--core=0', '--', command, ...args],
    {cwd: repo, stdio: fds, env: {...process.env, ...env}});
  fds.forEach(fd => fs.closeSync(fd));
  const lines = fs.readFileSync(stem + '.time', 'utf8').trim().split('\n');
  return {status: result.status, metrics: JSON.parse(lines.at(-1)),
    stdout: fs.readFileSync(stem + '.stdout', 'utf8'), stderr: fs.readFileSync(stem + '.stderr', 'utf8')};
}
const sourceRows = fs.readFileSync(path.join(directory, 'balanced_wide.tsv'), 'utf8').trim().split('\n').slice(1)
  .map(line => {const fields = line.split('\t'); return {bits: Number(fields[1]), source: fields[2]};});
const widths = process.argv.slice(2).map(Number);
const sources = widths.length ? sourceRows.filter(row => widths.includes(row.bits)) : sourceRows;
if (!sources.length) throw Error('no requested balanced source');
const results = [];
for (const row of sources) {
  const prepare = run('words_' + row.bits, 'target/debug/ququart_prepare_operator', ['--numerals', row.source, '4', '1729']);
  if (prepare.status !== 0) throw Error(prepare.stderr);
  const words = JSON.parse(prepare.stdout);
  const bound = {source_word: words[row.source], base_word: words['4'], radix_word: words['4'], seed_word: words['1729'],
    phase_clock: clocks.find(c => c.retained === 0 && c.seed === 1), readout_clocks: clocks.filter(c => c.seed === 1)};
  const preparedFile = path.join(output, 'prepared_' + row.bits + '.json');
  fs.writeFileSync(preparedFile, JSON.stringify(bound, null, 2) + '\n');
  const build = run('build_' + row.bits, 'cargo', ['build', '-j1', '--bin', 'ququart_work_probe_baked'],
    {QUQUART_PREPARED_FILE: preparedFile}, 60);
  if (build.status !== 0) throw Error(build.stderr);
  const binary = path.join(repo, 'target/debug/ququart_work_probe_baked');
  const preparedBytes = fs.readFileSync(preparedFile), binaryBytes = fs.readFileSync(binary);
  if (!binaryBytes.includes(preparedBytes)) throw Error('prepared source/clock not baked in executable');
  const execution = run('execute_' + row.bits, binary, [], {}, 30);
  const result = {bits: row.bits, source: row.source, status: execution.status, metrics: execution.metrics,
    executable_sha256: crypto.createHash('sha256').update(binaryBytes).digest('hex'),
    prepared_sha256: crypto.createHash('sha256').update(preparedBytes).digest('hex'),
    baked_bytes_present: true, report: execution.status === 0 ? JSON.parse(execution.stdout) : null,
    error: execution.stderr.trim()};
  results.push(result);
  fs.writeFileSync(path.join(output, 'executions.json'), JSON.stringify(results, null, 2) + '\n');
  console.log(JSON.stringify({bits: row.bits, status: result.status, metrics: result.metrics}));
  if (execution.status !== 0) break;
}
