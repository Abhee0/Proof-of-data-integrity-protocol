/* ============================================================
   PODIP — Proof of Data Integrity Protocol
   Frontend Application Logic
   ============================================================ */

'use strict';

// ── NAVBAR SCROLL ────────────────────────────────────────────────────────────
(function initNavbar() {
  const navbar = document.getElementById('navbar');
  const onScroll = () => {
    navbar.classList.toggle('scrolled', window.scrollY > 40);
  };
  window.addEventListener('scroll', onScroll, { passive: true });
})();

// ── NETWORK CANVAS ANIMATION ─────────────────────────────────────────────────
(function initCanvas() {
  const canvas = document.getElementById('network-canvas');
  if (!canvas) return;
  const ctx = canvas.getContext('2d');

  let nodes = [];
  let animFrame;
  const NODE_COUNT = 55;
  const MAX_DIST = 160;
  const SPEED = 0.35;

  function resize() {
    canvas.width  = canvas.offsetWidth;
    canvas.height = canvas.offsetHeight;
  }

  function buildNodes() {
    nodes = [];
    for (let i = 0; i < NODE_COUNT; i++) {
      nodes.push({
        x:  Math.random() * canvas.width,
        y:  Math.random() * canvas.height,
        vx: (Math.random() - 0.5) * SPEED,
        vy: (Math.random() - 0.5) * SPEED,
        r:  Math.random() * 2 + 1.2,
      });
    }
  }

  function draw() {
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    // Update positions
    for (const n of nodes) {
      n.x += n.vx;
      n.y += n.vy;
      if (n.x < 0 || n.x > canvas.width)  n.vx *= -1;
      if (n.y < 0 || n.y > canvas.height) n.vy *= -1;
    }

    // Draw edges
    for (let i = 0; i < nodes.length; i++) {
      for (let j = i + 1; j < nodes.length; j++) {
        const a = nodes[i], b = nodes[j];
        const dx = a.x - b.x, dy = a.y - b.y;
        const dist = Math.sqrt(dx * dx + dy * dy);
        if (dist < MAX_DIST) {
          const alpha = (1 - dist / MAX_DIST) * 0.35;
          // Gradient: teal → purple
          const grad = ctx.createLinearGradient(a.x, a.y, b.x, b.y);
          grad.addColorStop(0, `rgba(110, 231, 247, ${alpha})`);
          grad.addColorStop(1, `rgba(167, 139, 250, ${alpha})`);
          ctx.beginPath();
          ctx.moveTo(a.x, a.y);
          ctx.lineTo(b.x, b.y);
          ctx.strokeStyle = grad;
          ctx.lineWidth = 0.8;
          ctx.stroke();
        }
      }
    }

    // Draw nodes
    for (const n of nodes) {
      ctx.beginPath();
      ctx.arc(n.x, n.y, n.r, 0, Math.PI * 2);
      ctx.fillStyle = 'rgba(110, 231, 247, 0.55)';
      ctx.fill();
    }

    animFrame = requestAnimationFrame(draw);
  }

  const ro = new ResizeObserver(() => {
    resize();
    buildNodes();
  });
  ro.observe(canvas.parentElement);
  resize();
  buildNodes();
  draw();
})();

// ── SCROLL REVEAL ─────────────────────────────────────────────────────────────
(function initScrollReveal() {
  const els = document.querySelectorAll(
    '.demo-card, .tech-card, .threat-card, .gas-card, .usecase-card, ' +
    '.section-header, .flow-diagram, .arch-diagram, .roadmap-table-wrap, ' +
    '.opt-title, .limitations-box, .optimization-note'
  );

  els.forEach(el => el.classList.add('reveal'));

  const io = new IntersectionObserver((entries) => {
    entries.forEach(e => {
      if (e.isIntersecting) {
        e.target.classList.add('visible');
        io.unobserve(e.target);
      }
    });
  }, { threshold: 0.08 });

  els.forEach(el => io.observe(el));
})();

// ── HOW IT WORKS TABS ─────────────────────────────────────────────────────────
(function initTabs() {
  const tabs      = document.querySelectorAll('.flow-tab');
  const storeFlow = document.getElementById('flow-store');
  const verifyFlow = document.getElementById('flow-verify');

  tabs.forEach(tab => {
    tab.addEventListener('click', () => {
      tabs.forEach(t => t.classList.remove('active'));
      tab.classList.add('active');

      const which = tab.dataset.tab;
      if (which === 'store') {
        storeFlow.classList.remove('hidden');
        verifyFlow.classList.add('hidden');
      } else {
        storeFlow.classList.add('hidden');
        verifyFlow.classList.remove('hidden');
      }
    });
  });
})();

// ── SHA-256 HASHING (WebCrypto) ───────────────────────────────────────────────
async function sha256File(file, onProgress) {
  // We stream the file in 2 MiB chunks and update a progress bar.
  // WebCrypto doesn't natively support streaming SHA-256, so we accumulate
  // and hash at the end — but show read-progress to the user.
  const CHUNK = 2 * 1024 * 1024; // 2 MiB
  const totalChunks = Math.ceil(file.size / CHUNK);
  const parts = [];

  for (let i = 0; i < totalChunks; i++) {
    const start = i * CHUNK;
    const end   = Math.min(start + CHUNK, file.size);
    const blob  = file.slice(start, end);
    const buf   = await blob.arrayBuffer();
    parts.push(new Uint8Array(buf));
    onProgress && onProgress((i + 1) / totalChunks);
  }

  // Combine all chunks
  const totalLen = parts.reduce((sum, p) => sum + p.length, 0);
  const combined = new Uint8Array(totalLen);
  let offset = 0;
  for (const p of parts) {
    combined.set(p, offset);
    offset += p.length;
  }

  const hashBuf = await crypto.subtle.digest('SHA-256', combined);
  const hashArr = Array.from(new Uint8Array(hashBuf));
  return '0x' + hashArr.map(b => b.toString(16).padStart(2, '0')).join('');
}

function formatBytes(bytes) {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

// ── DROP ZONE + FILE HASHING ──────────────────────────────────────────────────
(function initDropZone() {
  const dropZone   = document.getElementById('drop-zone');
  const fileInput  = document.getElementById('file-input');
  const browseBtn  = document.getElementById('browse-btn');
  const hashResult = document.getElementById('hash-result');
  const progressWrap = document.getElementById('progress-wrap');
  const progressBar  = document.getElementById('progress-bar');
  const hashString   = document.getElementById('hash-string');
  const fileNameDisplay = document.getElementById('file-name-display');
  const fileSizeDisplay = document.getElementById('file-size-display');
  const hashMeta       = document.getElementById('hash-meta');
  const copyBtn        = document.getElementById('copy-btn');
  const resetBtn       = document.getElementById('reset-btn');

  // Click-to-browse
  browseBtn.addEventListener('click', () => fileInput.click());
  dropZone.addEventListener('click', (e) => {
    if (e.target !== browseBtn) fileInput.click();
  });

  // Drag-and-drop
  dropZone.addEventListener('dragover', (e) => {
    e.preventDefault();
    dropZone.classList.add('drag-over');
  });
  ['dragleave', 'dragend'].forEach(ev =>
    dropZone.addEventListener(ev, () => dropZone.classList.remove('drag-over'))
  );
  dropZone.addEventListener('drop', (e) => {
    e.preventDefault();
    dropZone.classList.remove('drag-over');
    const files = e.dataTransfer.files;
    if (files.length) processFile(files[0]);
  });

  // File input change
  fileInput.addEventListener('change', () => {
    if (fileInput.files.length) processFile(fileInput.files[0]);
  });

  async function processFile(file) {
    // Show result panel
    dropZone.classList.add('hidden');
    hashResult.classList.remove('hidden');
    progressWrap.classList.remove('hidden');
    progressBar.style.width = '0%';

    fileNameDisplay.textContent = file.name;
    fileSizeDisplay.textContent = formatBytes(file.size);
    hashString.textContent = 'Computing…';
    hashMeta.textContent   = '';

    const startTime = performance.now();

    try {
      const hash = await sha256File(file, (progress) => {
        progressBar.style.width = (progress * 100).toFixed(1) + '%';
      });

      const elapsed = ((performance.now() - startTime) / 1000).toFixed(2);
      const speed   = (file.size / (1024 * 1024) / parseFloat(elapsed)).toFixed(1);

      progressBar.style.width = '100%';
      setTimeout(() => progressWrap.classList.add('hidden'), 600);

      hashString.textContent = hash;
      hashMeta.textContent = `Computed in ${elapsed}s · ${speed} MB/s · ${file.size.toLocaleString()} bytes read`;

    } catch (err) {
      hashString.textContent = 'Error: ' + err.message;
      hashString.style.color = 'var(--red)';
    }
  }

  // Copy button
  copyBtn.addEventListener('click', async () => {
    const text = hashString.textContent;
    if (!text || text === 'Computing…') return;
    try {
      await navigator.clipboard.writeText(text);
      copyBtn.classList.add('copied');
      copyBtn.innerHTML = `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="20 6 9 17 4 12"/></svg>`;
      setTimeout(() => {
        copyBtn.classList.remove('copied');
        copyBtn.innerHTML = `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>`;
      }, 2000);
    } catch {
      /* clipboard not available in non-secure contexts */
    }
  });

  // Reset
  resetBtn.addEventListener('click', () => {
    hashResult.classList.add('hidden');
    dropZone.classList.remove('hidden');
    fileInput.value = '';
    hashString.textContent   = 'Computing…';
    hashString.style.color   = '';
    progressBar.style.width  = '0%';
    progressWrap.classList.add('hidden');
  });
})();

// ── SIMULATED BLOCKCHAIN VERIFY ───────────────────────────────────────────────
(function initVerify() {
  const hashInput   = document.getElementById('verify-hash-input');
  const verifyBtn   = document.getElementById('verify-btn');
  const resultBox   = document.getElementById('verify-result');
  const statusBox   = document.getElementById('verify-status-box');
  const statusIcon  = document.getElementById('verify-status-icon');
  const statusText  = document.getElementById('verify-status-text');
  const detailsBox  = document.getElementById('verify-details');
  const quickFound  = document.getElementById('quick-found');
  const quickNotFound = document.getElementById('quick-not-found');

  const detailTimestamp = document.getElementById('detail-timestamp');
  const detailUploader  = document.getElementById('detail-uploader');
  const detailFilename  = document.getElementById('detail-filename');
  const detailBlock     = document.getElementById('detail-block');

  // Simulated on-chain records (what the deployed contract would return)
  const RECORDS = {
    '0x3a4b5c6d8f1e9a2b7c4d5e6f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b': {
      filename:  'contract_draft.pdf',
      uploader:  '0xAbCd1234EF567890aBcD1234eF567890AbCd1234',
      timestamp: 1731677387,
      block:     5812340,
    },
    '0xc4a9ef3d2819a7bc56f3e1d892b4a6c71e8f25d09b3a7412c6d5e8f910b2c3d4': {
      filename:  'research_data_q3_2024.csv',
      uploader:  '0xDEAD1234Beef5678Dead1234bEeF5678dEaD1234',
      timestamp: 1729083600,
      block:     5799001,
    },
    '0x91a2b3c4d5e6f70809aabbccddeeff0011223344556677889900aabbccddeeff00': {
      filename:  'audit_log_2024-10-15.json',
      uploader:  '0x1234AbCdEf5678901234abcdef5678901234AbCd',
      timestamp: 1728950400,
      block:     5780500,
    },
  };

  function formatTimestamp(unix) {
    const d = new Date(unix * 1000);
    return d.toUTCString().replace('GMT', 'UTC') + ` (${unix} unix)`;
  }

  function truncate(addr) {
    return addr.slice(0, 10) + '…' + addr.slice(-8);
  }

  function isValidHash(v) {
    return /^0x[0-9a-fA-F]{64}$/.test(v.trim());
  }

  // Enable verify button when there's valid input
  hashInput.addEventListener('input', () => {
    const v = hashInput.value.trim();
    verifyBtn.disabled = !isValidHash(v);
    if (v.length > 2 && !v.startsWith('0x')) {
      hashInput.value = '0x' + v;
    }
  });

  // Quick-fill buttons
  [quickFound, quickNotFound].forEach(btn => {
    btn.addEventListener('click', () => {
      hashInput.value = btn.dataset.hash;
      verifyBtn.disabled = false;
      resultBox.classList.add('hidden');
    });
  });

  // Verify
  verifyBtn.addEventListener('click', async () => {
    const hash = hashInput.value.trim().toLowerCase();
    if (!isValidHash(hash)) return;

    verifyBtn.disabled = true;
    verifyBtn.innerHTML = `
      <svg class="spin" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M21 12a9 9 0 1 1-6.219-8.56"/>
      </svg>
      Querying Simulated Chain…
    `;

    // Simulate network latency
    await delay(900 + Math.random() * 600);

    const record = RECORDS[hash] || null;
    showResult(record, hash);

    verifyBtn.disabled = false;
    verifyBtn.innerHTML = `
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/>
      </svg>
      Verify on Simulated Chain
    `;
  });

  function showResult(record, hash) {
    resultBox.classList.remove('hidden');

    if (record) {
      statusBox.className = 'verify-status found';
      statusIcon.textContent = '✅';
      statusText.textContent = 'VERIFIED — proof exists on-chain';

      detailsBox.classList.remove('hidden');
      detailTimestamp.textContent = formatTimestamp(record.timestamp);
      detailUploader.textContent  = truncate(record.uploader);
      detailFilename.textContent  = record.filename;
      detailBlock.textContent     = '#' + record.block.toLocaleString();
    } else {
      statusBox.className = 'verify-status not-found';
      statusIcon.textContent = '❌';
      statusText.textContent = 'NOT FOUND — no proof exists for this hash';
      detailsBox.classList.add('hidden');
    }

    // Trigger animation restart
    statusBox.style.animation = 'none';
    requestAnimationFrame(() => {
      statusBox.style.animation = '';
    });
  }
})();

// ── SMOOTH SCROLL FOR NAV LINKS ───────────────────────────────────────────────
(function initSmoothScroll() {
  document.querySelectorAll('a[href^="#"]').forEach(link => {
    link.addEventListener('click', (e) => {
      const target = document.querySelector(link.getAttribute('href'));
      if (!target) return;
      e.preventDefault();
      const offset = 70; // navbar height
      const top = target.getBoundingClientRect().top + window.scrollY - offset;
      window.scrollTo({ top, behavior: 'smooth' });
    });
  });
})();

// ── SPINNER CSS INJECTION ─────────────────────────────────────────────────────
(function injectSpinnerStyle() {
  const style = document.createElement('style');
  style.textContent = `
    @keyframes spin {
      from { transform: rotate(0deg); }
      to   { transform: rotate(360deg); }
    }
    .spin { animation: spin 0.9s linear infinite; }
  `;
  document.head.appendChild(style);
})();

// ── UTILITY ───────────────────────────────────────────────────────────────────
function delay(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}
