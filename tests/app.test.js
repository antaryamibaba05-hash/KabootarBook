import { readFile } from 'node:fs/promises';
import { describe, it } from 'node:test';
import assert from 'node:assert/strict';

const source = await readFile(new URL('../src/main.js', import.meta.url), 'utf8');
const storage = await readFile(new URL('../src/storage.js', import.meta.url), 'utf8');
const schema = await readFile(new URL('../src-tauri/migrations/001_init.sql', import.meta.url), 'utf8');

describe('KabootarBook app invariants', () => {
  it('derives durations from timestamp differences, not stored durations', () => {
    assert.match(source, /new Date\(f\.endedAt\)-new Date\(f\.startedAt\)/);
    assert.doesNotMatch(source, /duration:\s*fd\.get/);
  });
  it('keeps shared sessions as one flight record', () => {
    assert.match(source, /function totalFlights\(\)\{return state\.flights\.length\}/);
    assert.match(source, /const selected=fd\.getAll\('pigeonIds'\)/);
  });
  it('rejects a return earlier than release', () => {
    assert.match(source, /end<start/);
  });
  it('prevents duplicate ring numbers without case sensitivity', () => {
    assert.match(source, /p\.ring\.toLowerCase\(\)===ring\.toLowerCase\(\)/);
    assert.match(source, /That ring number is already in your loft/);
  });
  it('persists an active session and reloads it from SQLite', () => {
    assert.match(source, /state\.active=\{\.\.\.state\.active,\.\.\.obj\}/);
    assert.match(storage, /SELECT state_json FROM app_state/);
    assert.match(storage, /JSON\.parse\(rows\[0\]\.state_json\)/);
  });
  it('uses an atomic SQLite state row migration', () => {
    assert.match(schema, /CREATE TABLE IF NOT EXISTS app_state/);
    assert.match(schema, /state_json TEXT NOT NULL/);
  });
  it('writes full SQLite snapshots transactionally', () => {
    assert.match(storage, /ON CONFLICT\(id\) DO UPDATE/);
    assert.match(storage, /state_json/);
    assert.doesNotMatch(storage, /DELETE FROM pigeons|DELETE FROM flight_sessions/);
  });
  it('validates all imported records before appending', () => {
    assert.match(source, /Ring \$\{p\.ring\} already exists/);
    assert.match(source, /nothing imported/);
    assert.match(source, /state\.pigeons\.push\(\.\.\.incomingP\)/);
  });
});
