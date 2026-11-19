(function () {
  'use strict';

  var enc = new TextEncoder();
  var dec = new TextDecoder('utf-8');
  var wasm = null;
  var appEl = document.getElementById('app');
  var isBusyModal = false;
  var CURRENT_SEMVER = '2.1';

  /* Pure-JS SHA-256 for non-HTTPS LAN IPs (where window.crypto.subtle is unavailable) */
  var K256 = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
  ];

  function sha256HexBytes(bytes) {
    var len = bytes.length;
    var bitHi = Math.floor((len * 8) / 0x100000000);
    var bitLo = (len * 8) >>> 0;
    var padLen = ((len + 9 + 63) & ~63);
    var msg = new Uint8Array(padLen);
    msg.set(bytes);
    msg[len] = 0x80;
    var dv = new DataView(msg.buffer);
    dv.setUint32(padLen - 8, bitHi, false);
    dv.setUint32(padLen - 4, bitLo, false);

    var h0 = 0x6a09e667, h1 = 0xbb67ae85, h2 = 0x3c6ef372, h3 = 0xa54ff53a;
    var h4 = 0x510e527f, h5 = 0x9b05688c, h6 = 0x1f83d9ab, h7 = 0x5be0cd19;
    var w = new Int32Array(64);

    for (var off = 0; off < padLen; off += 64) {
      for (var i = 0; i < 16; i++) w[i] = dv.getInt32(off + i * 4, false);
      for (var j = 16; j < 64; j++) {
        var x = w[j - 15], y = w[j - 2];
        var s0 = ((x >>> 7) | (x << 25)) ^ ((x >>> 18) | (x << 14)) ^ (x >>> 3);
        var s1 = ((y >>> 17) | (y << 15)) ^ ((y >>> 19) | (y << 13)) ^ (y >>> 10);
        w[j] = (w[j - 16] + s0 + w[j - 7] + s1) | 0;
      }
      var a = h0, b = h1, c = h2, d = h3, e = h4, f = h5, g = h6, h = h7;
      for (var r = 0; r < 64; r++) {
        var S1 = ((e >>> 6) | (e << 26)) ^ ((e >>> 11) | (e << 21)) ^ ((e >>> 25) | (e << 7));
        var ch = (e & f) ^ ((~e) & g);
        var t1 = (h + S1 + ch + K256[r] + w[r]) | 0;
        var S0 = ((a >>> 2) | (a << 30)) ^ ((a >>> 13) | (a << 19)) ^ ((a >>> 22) | (a << 10));
        var maj = (a & b) ^ (a & c) ^ (b & c);
        var t2 = (S0 + maj) | 0;
        h = g; g = f; f = e; e = (d + t1) | 0;
        d = c; c = b; b = a; a = (t1 + t2) | 0;
      }
      h0 = (h0 + a) | 0; h1 = (h1 + b) | 0; h2 = (h2 + c) | 0; h3 = (h3 + d) | 0;
      h4 = (h4 + e) | 0; h5 = (h5 + f) | 0; h6 = (h6 + g) | 0; h7 = (h7 + h) | 0;
    }
    function hex32(v) { return ('00000000' + (v >>> 0).toString(16)).slice(-8); }
    return hex32(h0) + hex32(h1) + hex32(h2) + hex32(h3) + hex32(h4) + hex32(h5) + hex32(h6) + hex32(h7);
  }

  function toWasmStr(str) {
    var s = String(str || '');
    if (!s) return { ptr: 0, len: 0, cap: 0 };
    var bytes = enc.encode(s);
    var ptr = wasm.wasm_alloc(bytes.length);
    var mem = new Uint8Array(wasm.memory.buffer, ptr, bytes.length);
    mem.set(bytes);
    return { ptr: ptr, len: bytes.length, cap: bytes.length };
  }

  function freeWasmStr(buf) {
    if (buf && buf.ptr && buf.cap) {
      wasm.wasm_dealloc(buf.ptr, buf.cap);
    }
  }

  function readWasmOutput(ptr) {
    var len = wasm.wasm_output_len();
    if (!ptr || !len) return '';
    return dec.decode(new Uint8Array(wasm.memory.buffer, ptr, len));
  }

  function renderFull() {
    if (!wasm || isBusyModal) return;
    var ptr = wasm.wasm_render_app();
    appEl.innerHTML = readWasmOutput(ptr);
  }

  function setNotice(kind, msg) {
    if (!wasm) return;
    var k = toWasmStr(kind);
    var m = toWasmStr(msg);
    var ptr = wasm.wasm_set_notice(k.ptr, k.len, m.ptr, m.len);
    freeWasmStr(k);
    freeWasmStr(m);
    appEl.innerHTML = readWasmOutput(ptr);
  }

  function handleSessionExpired() {
    if (!wasm) return;
    var raw = toWasmStr('{"ok":true,"authenticated":false}');
    var ptr = wasm.wasm_ingest_auth(raw.ptr, raw.len);
    freeWasmStr(raw);
    appEl.innerHTML = readWasmOutput(ptr);
    setNotice('info', 'Your session has expired. Please sign in again.');
  }

  function showWaitOverlay(title, subtitle, seconds, redirectUrl) {
    isBusyModal = true;
    var overlay = document.createElement('div');
    overlay.className = 'modal-backdrop';
    var remain = Math.max(1, seconds || 4);
    function updateModal() {
      var pct = Math.min(100, Math.round(((seconds - remain) / Math.max(1, seconds)) * 100));
      overlay.innerHTML =
        '<div class="modal-card">' +
          '<h3 class="modal-title">' + title + '</h3>' +
          '<p class="modal-sub">' + subtitle + ' (' + remain + 's)</p>' +
          '<div class="progress-track"><div class="progress-bar" style="width:' + pct + '%"></div></div>' +
        '</div>';
    }
    updateModal();
    document.body.appendChild(overlay);

    var timer = setInterval(function () {
      remain--;
      if (remain <= 0) {
        clearInterval(timer);
        if (redirectUrl) {
          window.location.href = redirectUrl;
          return;
        }
        document.body.removeChild(overlay);
        isBusyModal = false;
        pollTelemetry(true, function () {
          setNotice('ok', 'Configuration applied and saved to NVRAM.');
        });
      } else {
        updateModal();
      }
    }, 1000);
  }

  function fetchAuthText(url, opts) {
    return fetch(url, Object.assign({ credentials: 'same-origin', cache: 'no-store' }, opts || {}))
      .then(function (r) {
        if (r.status === 401) {
          handleSessionExpired();
          return null;
        }
        return r.ok ? r.text() : '';
      })
      .catch(function () { return ''; });
  }

  function fetchUsers(doneCb) {
    if (!wasm || wasm.wasm_is_authenticated() !== 1) return;
    fetch('/api_users.cgi?_=' + Date.now(), { credentials: 'same-origin', cache: 'no-store' })
      .then(function (r) {
        if (r.status === 401) {
          handleSessionExpired();
          return '';
        }
        return r.text();
      })
      .then(function (txt) {
        if (!txt) return;
        var b = toWasmStr(txt);
        var ptr = wasm.wasm_ingest_users(b.ptr, b.len);
        freeWasmStr(b);
        appEl.innerHTML = readWasmOutput(ptr);
        if (doneCb) doneCb();
      })
      .catch(function () {});
  }

  function checkSessionAndInit(noticeAfter) {
    fetch('/api_auth.cgi?_=' + Date.now(), { credentials: 'same-origin', cache: 'no-store' })
      .then(function (r) { return r.text(); })
      .then(function (txt) {
        var b = toWasmStr(txt || '{"ok":true,"authenticated":false}');
        var ptr = wasm.wasm_ingest_auth(b.ptr, b.len);
        freeWasmStr(b);
        appEl.innerHTML = readWasmOutput(ptr);
        if (noticeAfter) {
          setNotice(noticeAfter.kind, noticeAfter.msg);
        }
        if (wasm.wasm_is_authenticated() === 1) {
          pollTelemetry(true);
          fetchUsers();
        }
      })
      .catch(function () {
        var b = toWasmStr('{"ok":true,"authenticated":false}');
        var ptr = wasm.wasm_ingest_auth(b.ptr, b.len);
        freeWasmStr(b);
        appEl.innerHTML = readWasmOutput(ptr);
      });
  }

  function handleLoginSubmit() {
    if (!wasm) return;
    var out = readWasmOutput(wasm.wasm_build_login_payload());
    var lines = out.split('\n');
    if (lines[0] !== 'OK') {
      setNotice('err', lines[1] || 'Please enter username and password.');
      return;
    }
    appEl.innerHTML = readWasmOutput(wasm.wasm_set_login_busy(1));
    fetch('/api_auth.cgi', {
      method: 'POST',
      credentials: 'same-origin',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: lines[1]
    })
      .then(function (r) { return r.text(); })
      .then(function (txt) {
        var parsed = null;
        try { parsed = JSON.parse(txt); } catch (e) { parsed = null; }
        if (parsed && parsed.ok && parsed.authenticated) {
          var b = toWasmStr(txt);
          var ptr = wasm.wasm_ingest_auth(b.ptr, b.len);
          freeWasmStr(b);
          appEl.innerHTML = readWasmOutput(ptr);
          pollTelemetry(true);
          fetchUsers();
        } else {
          wasm.wasm_set_login_busy(0);
          setNotice('err', (parsed && parsed.error) ? parsed.error : 'Invalid username or password.');
        }
      })
      .catch(function (err) {
        wasm.wasm_set_login_busy(0);
        setNotice('err', 'Login request failed: ' + err.message);
      });
  }

  function handleLogout() {
    fetch('/api_auth.cgi', {
      method: 'POST',
      credentials: 'same-origin',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: 'action=logout'
    }).finally(function () {
      checkSessionAndInit({ kind: 'ok', msg: 'Signed out successfully.' });
    });
  }

  function submitUserAction(bodyStr) {
    fetch('/api_users.cgi', {
      method: 'POST',
      credentials: 'same-origin',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: bodyStr
    })
      .then(function (r) {
        if (r.status === 401) {
          handleSessionExpired();
          return null;
        }
        return r.text();
      })
      .then(function (txt) {
        if (!txt) return;
        var parsed = null;
        try { parsed = JSON.parse(txt); } catch (e) { parsed = null; }
        if (parsed && parsed.ok) {
          var b = toWasmStr(txt);
          wasm.wasm_ingest_users(b.ptr, b.len);
          freeWasmStr(b);
          setNotice('ok', parsed.message || 'User accounts updated.');
        } else {
          setNotice('err', (parsed && parsed.error) ? parsed.error : 'Failed to update user account.');
        }
      })
      .catch(function (err) {
        setNotice('err', 'User management request failed: ' + err.message);
      });
  }

  function syncLocateBtnDom() {
    if (!wasm || isBusyModal) return;
    var curBtn = document.querySelector('[data-action="locate-ap"]');
    if (!curBtn || !wasm.wasm_render_locate_btn) return;
    var html = readWasmOutput(wasm.wasm_render_locate_btn());
    if (html) {
      curBtn.outerHTML = html;
    }
  }

  function handleLocateToggle(btn) {
    if (!wasm) return;
    var activeNow = btn.getAttribute('data-locate-active') === '1';
    var bodyStr = activeNow ? 'action=stop' : 'seconds=30';
    fetch('/api_locate.cgi', {
      method: 'POST',
      credentials: 'same-origin',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: bodyStr
    })
      .then(function (r) {
        if (r.status === 401) {
          handleSessionExpired();
          return null;
        }
        if (r.status === 403) {
          throw new Error('Administrator privileges required.');
        }
        return r.text();
      })
      .then(function (txt) {
        if (!txt) return;
        if (wasm.wasm_ingest_locate) {
          var b = toWasmStr(txt);
          wasm.wasm_ingest_locate(b.ptr, b.len);
          freeWasmStr(b);
        }
        syncLocateBtnDom();
      })
      .catch(function (err) {
        setNotice('err', 'Locate LED request failed: ' + err.message);
      });
  }

  function pollTelemetry(syncForms, doneCb) {
    if (!wasm || isBusyModal || wasm.wasm_is_authenticated() !== 1) return;
    Promise.all([
      fetchAuthText('/ap_state.asp?_=' + Date.now()),
      fetchAuthText('/api_telemetry.cgi?_=' + Date.now())
    ]).then(function (arr) {
      if (arr[0] === null || wasm.wasm_is_authenticated() !== 1) return;
      var bState = toWasmStr(arr[0]);
      wasm.wasm_ingest_telemetry(
        bState.ptr, bState.len,
        0, 0,
        0, 0,
        0, 0,
        syncForms ? 1 : 0
      );
      freeWasmStr(bState);

      if (arr[1] && wasm.wasm_ingest_v2_telemetry) {
        var bV2 = toWasmStr(arr[1]);
        wasm.wasm_ingest_v2_telemetry(bV2.ptr, bV2.len, syncForms ? 1 : 0);
        freeWasmStr(bV2);
      }

      if (syncForms || wasm.wasm_current_tab_is_dashboard() === 1) {
        renderFull();
      } else {
        syncLocateBtnDom();
      }
      if (doneCb) doneCb();
    });
  }

  function submitStartApply(bodyStr, waitFallbackSec, redirectIp) {
    fetch('/start_apply.htm', {
      method: 'POST',
      credentials: 'same-origin',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: bodyStr
    })
      .then(function (r) {
        if (r.status === 401) {
          handleSessionExpired();
          return null;
        }
        if (r.status === 403) {
          throw new Error('Administrator privileges required.');
        }
        return r.text();
      })
      .then(function (html) {
        if (html === null) return;
        var m = html.match(/restart_needed_time\((\d+)\)/);
        var waitSec = m ? Math.max(2, parseInt(m[1], 10) + 1) : (waitFallbackSec || 4);
        var nextUrl = redirectIp ? ('http://' + redirectIp + '/') : '';
        showWaitOverlay('Applying Configuration', 'Committing NVRAM and updating services', waitSec, nextUrl);
      })
      .catch(function (err) {
        setNotice('err', 'Failed to apply settings: ' + err.message);
      });
  }

  function uploadBinaryBlobToUpgradeCgi(blob, fileName, progressPrefix) {
    isBusyModal = true;
    var overlay = document.createElement('div');
    overlay.className = 'modal-backdrop';
    overlay.innerHTML =
      '<div class="modal-card">' +
        '<h3 class="modal-title">Uploading Firmware</h3>' +
        '<p class="modal-sub" id="fwModalSub">' + (progressPrefix || 'Transferring image to RAM...') + ' 0%</p>' +
        '<div class="progress-track"><div class="progress-bar" id="fwProgBar" style="width:0%"></div></div>' +
      '</div>';
    document.body.appendChild(overlay);

    var fd = new FormData();
    fd.append('file', blob, fileName || 'Concard_N9_CSB334A-2700.web');

    var xhr = new XMLHttpRequest();
    xhr.open('POST', '/upgrade.cgi', true);
    xhr.upload.onprogress = function (ev) {
      if (ev.lengthComputable) {
        var pct = Math.round((ev.loaded / ev.total) * 100);
        var sub = document.getElementById('fwModalSub');
        var bar = document.getElementById('fwProgBar');
        if (sub) sub.textContent = (progressPrefix || 'Transferring image to RAM...') + ' ' + pct + '%';
        if (bar) bar.style.width = pct + '%';
      }
    };
    xhr.onload = function () {
      document.body.removeChild(overlay);
      isBusyModal = false;
      if (xhr.status === 401) {
        handleSessionExpired();
      } else if (xhr.status === 403) {
        setNotice('err', 'Administrator privileges required to flash firmware.');
      } else if (xhr.responseText && xhr.responseText.indexOf('UpdateError') !== -1) {
        setNotice('err', 'Firmware verification rejected: only genuine ConCard CSB334A-2700 images are accepted.');
      } else {
        showWaitOverlay('Flashing Firmware to SPI NOR', 'Writing mtd4 and rebooting Access Point', 68, window.location.pathname);
      }
    };
    xhr.onerror = function () {
      document.body.removeChild(overlay);
      isBusyModal = false;
      setNotice('err', 'Network error while uploading firmware.');
    };
    xhr.send(fd);
  }

  function handleFirmwareUpload() {
    var fileInp = document.getElementById('fwFileInput');
    if (!fileInp || !fileInp.files || !fileInp.files[0]) {
      setNotice('err', 'Please choose a .web or .trx firmware image file first.');
      return;
    }
    var file = fileInp.files[0];
    if (file.size > 7012352) {
      setNotice('err', 'Firmware image exceeds 6.68 MiB (6,848 KB) mtd4 flash partition limit.');
      return;
    }
    if (!window.confirm('Flash firmware "' + file.name + '" (' + (file.size / 1048576).toFixed(2) + ' MiB)? Do not power off the Access Point during flashing.')) {
      return;
    }
    uploadBinaryBlobToUpgradeCgi(file, file.name, 'Transferring image to RAM...');
  }

  /* ---------------- Dual-Path GitHub OTA Update (Direct AP + Browser Relay Fallback) ---------------- */

  function parseGithubOwnerRepo(repoUrl) {
    var s = String(repoUrl || '').trim()
      .replace(/^https?:\/\/github\.com\//i, '')
      .replace(/\.git$/i, '')
      .replace(/\/releases.*$/i, '')
      .replace(/\/+$/, '');
    var parts = s.split('/');
    if (parts.length >= 2 && parts[0] && parts[1]) {
      return parts[0] + '/' + parts[1];
    }
    return '';
  }

  function isRemoteSemverNewer(remoteVer, curVer) {
    var cleanA = String(remoteVer || '').replace(/^[^0-9]*/, '');
    var cleanB = String(curVer || '').replace(/^[^0-9]*/, '');
    if (!cleanA) return false;
    var pa = cleanA.split('.').map(function (n) { return parseInt(n, 10) || 0; });
    var pb = cleanB.split('.').map(function (n) { return parseInt(n, 10) || 0; });
    var m = Math.max(pa.length, pb.length);
    for (var i = 0; i < m; i++) {
      var va = pa[i] || 0, vb = pb[i] || 0;
      if (va > vb) return true;
      if (va < vb) return false;
    }
    return false;
  }

  function applyOtaJsonToWasm(obj) {
    if (!wasm || !wasm.wasm_ingest_ota_json) return;
    var str = typeof obj === 'string' ? obj : JSON.stringify(obj);
    var b = toWasmStr(str);
    var ptr = wasm.wasm_ingest_ota_json(b.ptr, b.len);
    freeWasmStr(b);
    appEl.innerHTML = readWasmOutput(ptr);
  }

  function browserRelayGithubCheck(repoUrl) {
    var ownerRepo = parseGithubOwnerRepo(repoUrl);
    if (!ownerRepo) {
      applyOtaJsonToWasm({ ok: false, error: 'Invalid GitHub repository URL. Expected https://github.com/<owner>/<repo>.' });
      return;
    }
    var normRepo = 'https://github.com/' + ownerRepo;

    function tryManifestBranch(br) {
      return fetch('https://raw.githubusercontent.com/' + ownerRepo + '/' + br + '/manifest.json?_=' + Date.now(), { cache: 'no-store' })
        .then(function (r) { return r.ok ? r.json() : null; })
        .catch(function () { return null; });
    }

    tryManifestBranch('main')
      .then(function (mf) { return mf || tryManifestBranch('master'); })
      .then(function (mf) {
        if (mf && (mf.version || mf.downloadUrl)) {
          var latV = String(mf.version || '');
          applyOtaJsonToWasm({
            ok: true,
            viaBrowserRelay: true,
            repoUrl: normRepo,
            latestVersion: latV,
            latestBuild: String(mf.buildCode || ''),
            hasUpdate: isRemoteSemverNewer(latV, CURRENT_SEMVER),
            fileName: String(mf.fileName || 'Concard_N9_CSB334A-2700.web'),
            downloadUrl: String(mf.downloadUrl || ''),
            sha256: String(mf.sha256 || ''),
            sizeBytes: Number(mf.sizeBytes) || 0,
            notes: String(mf.notes || '')
          });
          return null;
        }
        return fetch('https://api.github.com/repos/' + ownerRepo + '/releases/latest', {
          headers: { 'Accept': 'application/vnd.github+json' },
          cache: 'no-store'
        });
      })
      .then(function (relResp) {
        if (!relResp) return;
        if (relResp.status === 404) {
          applyOtaJsonToWasm({
            ok: true,
            viaBrowserRelay: true,
            repoUrl: normRepo,
            noReleaseYet: true,
            hasUpdate: false,
            notes: 'Repository is reachable, and no firmware release or manifest.json has been published yet.'
          });
          return;
        }
        if (!relResp.ok) {
          throw new Error('GitHub API returned HTTP ' + relResp.status);
        }
        return relResp.json().then(function (rel) {
          var assets = Array.isArray(rel.assets) ? rel.assets : [];
          var fwAsset = null;
          for (var i = 0; i < assets.length; i++) {
            if (/\.(web|trx)$/i.test(assets[i].name || '')) {
              fwAsset = assets[i];
              break;
            }
          }
          var shaMatch = String(rel.body || '').match(/[0-9a-fA-F]{64}/);
          var tag = String(rel.tag_name || rel.name || '');
          var rawFallbackUrl = fwAsset
            ? ('https://raw.githubusercontent.com/' + ownerRepo + '/' + encodeURIComponent(tag) + '/releases/' + encodeURIComponent(fwAsset.name))
            : '';
          applyOtaJsonToWasm({
            ok: true,
            viaBrowserRelay: true,
            repoUrl: normRepo,
            latestVersion: tag,
            latestBuild: '',
            hasUpdate: isRemoteSemverNewer(tag, CURRENT_SEMVER),
            fileName: fwAsset ? fwAsset.name : '',
            downloadUrl: fwAsset ? (fwAsset.browser_download_url || rawFallbackUrl) : '',
            sha256: shaMatch ? shaMatch[0].toLowerCase() : '',
            sizeBytes: fwAsset ? (Number(fwAsset.size) || 0) : 0,
            notes: String(rel.name || '')
          });
        });
      })
      .catch(function (err) {
        applyOtaJsonToWasm({
          ok: false,
          error: 'Could not reach GitHub repository (' + err.message + '). Verify repository URL and visibility.'
        });
      });
  }

  function handleSaveOtaRepo() {
    if (!wasm || !wasm.wasm_get_ota_repo_url) return;
    var repoUrl = readWasmOutput(wasm.wasm_get_ota_repo_url());
    fetch('/api_ota.cgi', {
      method: 'POST',
      credentials: 'same-origin',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: 'action=save_repo&repo_url=' + encodeURIComponent(repoUrl)
    })
      .then(function (r) {
        if (r.status === 401) { handleSessionExpired(); return null; }
        return r.text();
      })
      .then(function (txt) {
        if (!txt) return;
        var p = null;
        try { p = JSON.parse(txt); } catch (_) {}
        if (p && p.ok) {
          setNotice('ok', 'Saved GitHub OTA repository URL to NVRAM.');
        } else {
          setNotice('err', (p && p.error) ? p.error : 'Failed to save GitHub repository URL.');
        }
      })
      .catch(function (err) {
        setNotice('err', 'Failed to save GitHub repository URL: ' + err.message);
      });
  }

  function handleCheckGithubOta() {
    if (!wasm || !wasm.wasm_get_ota_repo_url) return;
    var repoUrl = readWasmOutput(wasm.wasm_get_ota_repo_url());
    appEl.innerHTML = readWasmOutput(wasm.wasm_set_ota_checking(1));

    fetch('/api_ota.cgi', {
      method: 'POST',
      credentials: 'same-origin',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: 'action=check&repo_url=' + encodeURIComponent(repoUrl)
    })
      .then(function (r) {
        if (r.status === 401) { handleSessionExpired(); return null; }
        return r.text();
      })
      .then(function (txt) {
        if (txt === null) return;
        var parsed = null;
        try { parsed = JSON.parse(txt); } catch (_) {}
        if (parsed && parsed.ok) {
          applyOtaJsonToWasm(parsed);
        } else {
          /* AP management VLAN lacks WAN/DNS -> seamlessly run Browser-Relayed GitHub check */
          browserRelayGithubCheck(repoUrl);
        }
      })
      .catch(function () {
        browserRelayGithubCheck(repoUrl);
      });
  }

  function handleRunGithubOta() {
    if (!wasm || !wasm.wasm_get_ota_download_info) return;
    var lines = readWasmOutput(wasm.wasm_get_ota_download_info()).split('\n');
    var dlUrl = (lines[0] || '').trim();
    var expSha = (lines[1] || '').trim().toLowerCase();
    var fileName = (lines[2] || 'Concard_N9_CSB334A-2700.web').trim();
    if (!dlUrl) {
      setNotice('err', 'No firmware asset download URL available for this release.');
      return;
    }
    if (!window.confirm('Download and flash "' + fileName + '" from GitHub now? Do not power off the Access Point during upgrade.')) {
      return;
    }

    setNotice('info', 'Requesting GitHub firmware download & signature verification...');
    fetch('/api_ota.cgi', {
      method: 'POST',
      credentials: 'same-origin',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: 'action=upgrade&download_url=' + encodeURIComponent(dlUrl) + '&sha256=' + encodeURIComponent(expSha)
    })
      .then(function (r) {
        if (r.status === 401) { handleSessionExpired(); return null; }
        return r.text();
      })
      .then(function (txt) {
        if (txt === null) return;
        var p = null;
        try { p = JSON.parse(txt); } catch (_) {}
        if (p && p.ok && p.flashing) {
          showWaitOverlay('Flashing GitHub Firmware to SPI NOR', p.message || 'Writing mtd4 and rebooting Access Point', 68, window.location.pathname);
          return;
        }
        if (p && !p.apOffline && p.error) {
          setNotice('err', p.error);
          return;
        }
        /* Browser-Relayed OTA Fallback: fetch firmware into browser RAM, verify CSB334A-2700 + SHA-256, push to /upgrade.cgi */
        setNotice('info', 'AP VLAN has no direct Internet route — downloading firmware via browser relay...');
        return fetch(dlUrl, { cache: 'no-store' })
          .then(function (fr) {
            if (!fr.ok) throw new Error('HTTP ' + fr.status + ' downloading firmware');
            return fr.arrayBuffer();
          })
          .then(function (ab) {
            var u8 = new Uint8Array(ab);
            if (u8.length < 2500000 || u8.length > 7012352) {
              throw new Error('Invalid image size (' + u8.length + ' bytes).');
            }
            if (u8[0] !== 0x27 || u8[1] !== 0x05 || u8[2] !== 0x19 || u8[3] !== 0x56) {
              throw new Error('Invalid uImage header magic.');
            }
            var pidStr = dec.decode(u8.subarray(36, 48)).replace(/\0+$/, '');
            if (pidStr !== 'CSB334A-2700') {
              throw new Error('ProductID mismatch (' + pidStr + '). Expected CSB334A-2700.');
            }
            if (expSha && expSha.length === 64) {
              var actualSha = sha256HexBytes(u8);
              if (actualSha !== expSha) {
                throw new Error('SHA-256 mismatch (' + actualSha.slice(0, 12) + ' != ' + expSha.slice(0, 12) + ').');
              }
            }
            var blob = new Blob([u8], { type: 'application/octet-stream' });
            uploadBinaryBlobToUpgradeCgi(blob, fileName, 'Relaying verified GitHub image to AP...');
          })
          .catch(function (relayErr) {
            setNotice('err', 'Browser-relayed download could not fetch binary directly (' + relayErr.message + '). If using GitHub Releases CORS, host raw binary or manifest.json in repo or upload manually.');
          });
      })
      .catch(function (err) {
        setNotice('err', 'GitHub OTA upgrade request failed: ' + err.message);
      });
  }

  appEl.addEventListener('input', function (ev) {
    var t = ev.target;
    if (!t || !wasm) return;
    var field = t.getAttribute('data-field');
    if (!field) return;
    var val = t.type === 'checkbox' ? (t.checked ? '1' : '0') : t.value;
    var fk = toWasmStr(field);
    var fv = toWasmStr(val);
    wasm.wasm_update_field(fk.ptr, fk.len, fv.ptr, fv.len);
    freeWasmStr(fk);
    freeWasmStr(fv);
  });

  appEl.addEventListener('change', function (ev) {
    var t = ev.target;
    if (!t || !wasm) return;
    var field = t.getAttribute('data-field');
    if (!field) return;
    var val = t.type === 'checkbox' ? (t.checked ? '1' : '0') : t.value;
    var fk = toWasmStr(field);
    var fv = toWasmStr(val);
    wasm.wasm_update_field(fk.ptr, fk.len, fv.ptr, fv.len);
    freeWasmStr(fk);
    freeWasmStr(fv);
    if (t.getAttribute('data-rerender') === '1') {
      renderFull();
    }
  });

  appEl.addEventListener('submit', function (ev) {
    ev.preventDefault();
    var form = ev.target;
    if (form && form.getAttribute('data-action') === 'login-submit') {
      handleLoginSubmit();
    }
  });

  appEl.addEventListener('click', function (ev) {
    var btn = ev.target.closest('[data-action]');
    if (!btn || !wasm) return;
    var act = btn.getAttribute('data-action');

    if (act === 'login') {
      ev.preventDefault();
      handleLoginSubmit();
    } else if (act === 'logout') {
      handleLogout();
    } else if (act === 'toggle-login-pass') {
      var lfk = toWasmStr('login_show_pass');
      var lfv = toWasmStr('1');
      wasm.wasm_update_field(lfk.ptr, lfk.len, lfv.ptr, lfv.len);
      freeWasmStr(lfk);
      freeWasmStr(lfv);
      renderFull();
    } else if (act === 'tab') {
      var tabName = btn.getAttribute('data-tab') || 'dashboard';
      var tabId = toWasmStr(tabName);
      var ptr = wasm.wasm_set_tab(tabId.ptr, tabId.len);
      freeWasmStr(tabId);
      appEl.innerHTML = readWasmOutput(ptr);
      if (tabName === 'users') {
        fetchUsers();
      }
    } else if (act === 'clear-notice') {
      setNotice('', '');
    } else if (act === 'refresh') {
      pollTelemetry(false);
    } else if (act === 'refresh-users') {
      fetchUsers();
    } else if (act === 'toggle-psk') {
      var fk = toWasmStr('w_show_psk');
      var fv = toWasmStr('1');
      wasm.wasm_update_field(fk.ptr, fk.len, fv.ptr, fv.len);
      freeWasmStr(fk);
      freeWasmStr(fv);
      renderFull();
    } else if (act === 'toggle-guest-psk') {
      var gfk = toWasmStr('wg_show_psk');
      var gfv = toWasmStr('1');
      wasm.wasm_update_field(gfk.ptr, gfk.len, gfv.ptr, gfv.len);
      freeWasmStr(gfk);
      freeWasmStr(gfv);
      renderFull();
    } else if (act === 'toggle-um-pass') {
      var ufk = toWasmStr('um_show_pass');
      var ufv = toWasmStr('1');
      wasm.wasm_update_field(ufk.ptr, ufk.len, ufv.ptr, ufv.len);
      freeWasmStr(ufk);
      freeWasmStr(ufv);
      renderFull();
    } else if (act === 'edit-user') {
      var uname = toWasmStr(btn.getAttribute('data-username') || '');
      var eptr = wasm.wasm_select_edit_user(uname.ptr, uname.len);
      freeWasmStr(uname);
      appEl.innerHTML = readWasmOutput(eptr);
    } else if (act === 'reset-user-form') {
      appEl.innerHTML = readWasmOutput(wasm.wasm_reset_user_form());
    } else if (act === 'save-user') {
      var uOut = readWasmOutput(wasm.wasm_build_user_save_payload());
      var uLines = uOut.split('\n');
      if (uLines[0] !== 'OK') {
        setNotice('err', uLines[1] || 'Please check user account fields.');
      } else {
        submitUserAction(uLines[1]);
      }
    } else if (act === 'delete-user') {
      var targetUser = btn.getAttribute('data-username') || '';
      if (!targetUser) return;
      if (!window.confirm('Delete user account "' + targetUser + '"? Active sessions for this user will be signed out immediately.')) return;
      submitUserAction('action=delete&username=' + encodeURIComponent(targetUser));
    } else if (act === 'save-wireless') {
      var wOut = readWasmOutput(wasm.wasm_build_wireless_apply());
      var wLines = wOut.split('\n');
      if (wLines[0] !== 'OK') {
        setNotice('err', wLines[1] || 'Invalid wireless parameters.');
      } else {
        submitStartApply(wLines[1], 4, '');
      }
    } else if (act === 'save-network') {
      var nOut = readWasmOutput(wasm.wasm_build_network_apply());
      var nLines = nOut.split('\n');
      if (nLines[0] !== 'OK') {
        setNotice('err', nLines[1] || 'Invalid network parameters.');
      } else {
        submitStartApply(nLines[2], 5, nLines[1] || '');
      }
    } else if (act === 'save-system') {
      var sOut = readWasmOutput(wasm.wasm_build_system_apply());
      var sLines = sOut.split('\n');
      if (sLines[0] !== 'OK') {
        setNotice('err', sLines[1] || 'Invalid system parameters.');
      } else {
        submitStartApply(sLines[1], 3, '');
      }
    } else if (act === 'reboot') {
      if (!window.confirm('Reboot this Access Point now? Wireless clients will reconnect in ~25 seconds.')) return;
      fetch('/apply.cgi', {
        method: 'POST',
        credentials: 'same-origin',
        headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
        body: 'action_mode=+Reboot+'
      }).finally(function () {
        showWaitOverlay('Rebooting Access Point', 'Waiting for Concard N9 v2.1 system startup', 28, window.location.pathname);
      });
    } else if (act === 'factory-reset') {
      if (!window.confirm('Restore all NVRAM settings to Factory Defaults and reboot?')) return;
      fetch('/apply.cgi', {
        method: 'POST',
        credentials: 'same-origin',
        headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
        body: 'action_mode=+RestoreNVRAM+'
      }).finally(function () {
        showWaitOverlay('Restoring Factory Defaults', 'Resetting NVRAM and rebooting device', 35, '/');
      });
    } else if (act === 'locate-ap') {
      handleLocateToggle(btn);
    } else if (act === 'upload-firmware') {
      handleFirmwareUpload();
    } else if (act === 'check-github-ota') {
      handleCheckGithubOta();
    } else if (act === 'save-ota-repo') {
      handleSaveOtaRepo();
    } else if (act === 'run-github-ota') {
      handleRunGithubOta();
    }
  });

  fetch('/ap_ui.wasm', { credentials: 'same-origin' })
    .then(function (r) { return r.arrayBuffer(); })
    .then(function (bytes) { return WebAssembly.instantiate(bytes, {}); })
    .then(function (res) {
      wasm = res.instance.exports;
      renderFull();
      checkSessionAndInit();
      setInterval(function () {
        if (wasm && wasm.wasm_is_authenticated() === 1) {
          pollTelemetry(false);
        }
      }, 5000);
      setInterval(function () {
        if (wasm && wasm.wasm_is_authenticated() === 1 && wasm.wasm_tick_locate) {
          var changed = wasm.wasm_tick_locate();
          if (changed === 1) {
            syncLocateBtnDom();
          }
        }
      }, 1000);
    })
    .catch(function (err) {
      appEl.innerHTML =
        '<div style="max-width:520px;margin:60px auto;padding:24px;background:#fff;border:1px solid #e2e8f0;border-radius:10px;text-align:center">' +
          '<h2 style="margin:0 0 8px;font-size:17px">Failed to load WebAssembly Management UI</h2>' +
          '<p style="color:#64748b;font-size:13px">' + String(err) + '</p>' +
        '</div>';
    });
})();
