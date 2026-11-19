/**
 * ConCard N9v1 Custom Firmware - Wi-Fi Roaming (802.11k/r) & FQ-CoDel Latency Benchmark
 * Runs natively on Windows test laptop (Intel Wireless-AC 9461) without extra dependencies.
 *
 * Modes:
 *   node benchmark_roaming_and_capacity.js status
 *   node benchmark_roaming_and_capacity.js latency <target_ip> [duration_sec]
 *   node benchmark_roaming_and_capacity.js roam-watch <gateway_or_peer_ip> [duration_sec]
 */

const { execFile, spawn } = require('child_process');

function parseNetshWlan(stdout) {
  const info = {};
  for (const rawLine of stdout.split(/\r?\n/)) {
    const idx = rawLine.indexOf(':');
    if (idx <= 0) continue;
    const key = rawLine.slice(0, idx).trim().toLowerCase();
    const val = rawLine.slice(idx + 1).trim();
    if (!val) continue;
    if (key === 'state') info.state = val;
    else if (key === 'ssid') info.ssid = val;
    else if (key === 'bssid' || key === 'ap bssid') info.bssid = val.toLowerCase();
    else if (key === 'radio type') info.radioType = val;
    else if (key === 'authentication') info.authentication = val;
    else if (key === 'cipher') info.cipher = val;
    else if (key === 'channel') info.channel = val;
    else if (key === 'signal') info.signal = val;
    else if (key.startsWith('receive rate')) info.rxRateMbps = val;
    else if (key.startsWith('transmit rate')) info.txRateMbps = val;
  }
  return info;
}

function getWlanStatus() {
  return new Promise((resolve) => {
    execFile('netsh', ['wlan', 'show', 'interfaces'], { timeout: 3000 }, (err, stdout) => {
      if (err) return resolve({ error: err.message });
      resolve(parseNetshWlan(stdout || ''));
    });
  });
}

function pingOnce(ip, timeoutMs = 400) {
  return new Promise((resolve) => {
    const t0 = performance.now();
    execFile('ping', ['-n', '1', '-w', String(timeoutMs), ip], { timeout: timeoutMs + 500 }, (err, stdout) => {
      const elapsed = performance.now() - t0;
      const out = String(stdout || '');
      const m = out.match(/time[=<]\s*(\d+)\s*ms/i);
      if (!err && (m || /TTL=/i.test(out))) {
        resolve({ ok: true, rttMs: m ? Number(m[1]) : Math.round(elapsed) });
      } else {
        resolve({ ok: false, rttMs: null });
      }
    });
  });
}

async function runLatencyTest(targetIp, durationSec = 15) {
  console.log(`[BENCHMARK] Measuring ICMP RTT & jitter to ${targetIp} for ${durationSec}s...`);
  const samples = [];
  let lost = 0;
  const endAt = Date.now() + durationSec * 1000;

  while (Date.now() < endAt) {
    const res = await pingOnce(targetIp, 500);
    if (res.ok) samples.push(res.rttMs);
    else lost++;
    await new Promise((r) => setTimeout(r, 150));
  }

  samples.sort((a, b) => a - b);
  const count = samples.length;
  const avg = count ? (samples.reduce((s, x) => s + x, 0) / count).toFixed(2) : null;
  const p50 = count ? samples[Math.floor(count * 0.5)] : null;
  const p95 = count ? samples[Math.floor(count * 0.95)] : null;
  const max = count ? samples[count - 1] : null;

  const summary = {
    targetIp,
    sent: count + lost,
    received: count,
    lost,
    lossPct: ((lost / Math.max(1, count + lost)) * 100).toFixed(2) + '%',
    minMs: count ? samples[0] : null,
    avgMs: avg ? Number(avg) : null,
    p50Ms: p50,
    p95Ms: p95,
    maxMs: max,
  };
  console.log(JSON.stringify(summary, null, 2));
}

async function runRoamWatch(targetIp, durationSec = 60) {
  console.log(`[ROAM-WATCH] Monitoring BSSID transitions & packet continuity to ${targetIp} for ${durationSec}s...`);
  let prevBssid = null;
  let outageStart = null;
  const roamEvents = [];
  const endAt = Date.now() + durationSec * 1000;

  while (Date.now() < endAt) {
    const [wlan, pingRes] = await Promise.all([getWlanStatus(), pingOnce(targetIp, 250)]);
    const nowIso = new Date().toISOString();

    if (!pingRes.ok && outageStart === null) {
      outageStart = performance.now();
    } else if (pingRes.ok && outageStart !== null) {
      const gapMs = Math.round(performance.now() - outageStart);
      console.log(`[${nowIso}] Packet continuity resumed (gap ~${gapMs} ms, RTT=${pingRes.rttMs} ms)`);
      outageStart = null;
    }

    if (wlan.bssid && prevBssid && wlan.bssid !== prevBssid) {
      const ev = {
        timestamp: nowIso,
        fromBssid: prevBssid,
        toBssid: wlan.bssid,
        channel: wlan.channel,
        signal: wlan.signal,
        pingOkDuringSample: pingRes.ok,
        rttMs: pingRes.rttMs,
      };
      roamEvents.push(ev);
      console.log(`[ROAM DETECTED] ${prevBssid} -> ${wlan.bssid} (Ch ${wlan.channel}, Signal ${wlan.signal}, RTT=${pingRes.rttMs}ms)`);
    }
    if (wlan.bssid) prevBssid = wlan.bssid;

    await new Promise((r) => setTimeout(r, 200));
  }

  console.log('=== ROAM WATCH SUMMARY ===');
  console.log(JSON.stringify({ finalWlan: await getWlanStatus(), roamCount: roamEvents.length, roamEvents }, null, 2));
}

(async () => {
  const mode = process.argv[2] || 'status';
  if (mode === 'status') {
    console.log(JSON.stringify(await getWlanStatus(), null, 2));
  } else if (mode === 'latency') {
    await runLatencyTest(process.argv[3] || '192.168.1.165', Number(process.argv[4] || 15));
  } else if (mode === 'roam-watch') {
    await runRoamWatch(process.argv[3] || '192.168.1.1', Number(process.argv[4] || 60));
  } else {
    console.error('Unknown mode. Use: status | latency <ip> [sec] | roam-watch <ip> [sec]');
    process.exit(1);
  }
})();
