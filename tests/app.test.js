import { readFile } from 'node:fs/promises';
import { describe, it } from 'node:test';
import assert from 'node:assert/strict';

const source = await readFile(new URL('../src/main.js', import.meta.url), 'utf8');
const storage = await readFile(new URL('../src/storage.js', import.meta.url), 'utf8');
const schema = await readFile(new URL('../src-tauri/migrations/001_init.sql', import.meta.url), 'utf8');
const html = await readFile(new URL('../index.html', import.meta.url), 'utf8');
const native = await readFile(new URL('../native/app/src/main/java/com/ranaadeeljameel/kabootarbook/MainActivity.kt', import.meta.url), 'utf8');
const rust = await readFile(new URL('../rust/src/lib.rs', import.meta.url), 'utf8');
const css = await readFile(new URL('../src/style.css', import.meta.url), 'utf8');

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
  it('keeps archived pigeons out of the active loft and provides archive access', () => {
    assert.match(source, /state\.pigeons\.filter\(p=>!p\.archived\)/);
    assert.match(source, /data-action="set-pigeon-view" data-view="archive"/);
    assert.match(source, /function openRetirePigeon/);
    assert.match(source, /archiveReason=reason/);
  });
  it('does not expose a delete-all-data setting', () => {
    assert.doesNotMatch(source, /Delete all local data|data-action="clear-data"/);
    assert.doesNotMatch(source, /case'clear-data'/);
  });
  it('uses Android inset variables to keep the UI clear of system bars', () => {
    assert.match(source, /safe-area-insets/);
    assert.match(css, /--safe-area-inset-top/);
    assert.match(css, /--safe-area-inset-bottom/);
  });
  it('reserves navigation height and Android bottom inset', () => {
    assert.match(css, /calc\(64px \+ var\(--safe-area-inset-bottom,0px\)\)/);
    assert.match(html, /viewport-fit=cover/);
  });
  it('native app is Kotlin Compose with Rust JNI, not a WebView frontend', () => {
    assert.match(native, /@Composable private fun KabootarBookApp/);
    assert.match(native, /System\.loadLibrary\("kabootarbook"\)/);
    assert.match(rust, /extern "system" fn Java_com_ranaadeeljameel_kabootarbook_MainActivity_nativeInit/);
    assert.doesNotMatch(native, /WebView|WebViewClient/);
  });
  it('native back dismisses dialogs or pops the previous screen', () => {
    assert.match(native, /fun goBack\(\).*?routeStack\.dropLast\(1\)/);
    assert.match(native, /BackHandler\(enabled=pendingDeleteFlight\.isNotEmpty\(\)\|\|pendingReturnPigeon\.isNotEmpty\(\)\|\|dialog\.isNotEmpty\(\)\|\|routeStack\.size>1\)/);
  });
  it('native flow persists active flights and individual pigeon landings', () => {
    assert.match(rust, /CREATE TABLE IF NOT EXISTS active_flight/);
    assert.match(rust, /A flight is already active/);
    assert.match(rust, /returned_at TEXT/);
    assert.match(rust, /nativeReturnPigeon/);
    assert.match(rust, /pigeon_returned_at/);
    assert.match(rust, /returned_at TEXT/);
    assert.match(native, /Record return/);
  });
  it('native pigeons have a full record profile route', () => {
    assert.match(native, /PigeonProfile\(p,flights,active\)/);
    assert.match(native, /function PigeonProfile|private fun PigeonProfile/);
    assert.match(native, /FLIGHT HISTORY/);
    assert.match(native, /Current session · flying/);
    assert.match(native, /Record return/);
  });
});
