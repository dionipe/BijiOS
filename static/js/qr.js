/**
 * DokuQR - Reusable QR Code Generator & Download Modal for DokuHub
 * Uses local client-side QRCode canvas rendering (works for LAN/private URLs too)
 * with PNG card export for easy sharing & direct mobile download.
 */

window.DokuQR = {
  async copyText(text) {
    if (!text) return false;
    try {
      if (navigator.clipboard && window.isSecureContext) {
        await navigator.clipboard.writeText(text);
        return true;
      }
    } catch { }
    // Reliable fallback for HTTP / LAN IP / iOS Safari
    try {
      const ta = document.createElement('textarea');
      ta.value = text;
      ta.setAttribute('readonly', '');
      ta.style.cssText = 'position:fixed; top:-9999px; left:-9999px; opacity:0;';
      document.body.appendChild(ta);
      ta.focus();
      ta.select();
      ta.setSelectionRange(0, text.length);
      const ok = document.execCommand('copy');
      document.body.removeChild(ta);
      return ok;
    } catch {
      return false;
    }
  },

  async renderCanvas(canvasEl, text, size = 180) {
    if (!canvasEl || !text) return;
    if (window.QRCode && typeof QRCode.toCanvas === 'function') {
      await QRCode.toCanvas(canvasEl, text, {
        width: size,
        margin: 2,
        color: { dark: '#0f172a', light: '#ffffff' }
      });
      return;
    }
    // Fallback if CDN script failed to load
    const ctx = canvasEl.getContext('2d');
    canvasEl.width = size;
    canvasEl.height = size;
    const img = new Image();
    img.crossOrigin = 'anonymous';
    img.onload = () => {
      ctx.fillStyle = '#ffffff';
      ctx.fillRect(0, 0, size, size);
      ctx.drawImage(img, 0, 0, size, size);
    };
    img.src = `https://api.qrserver.com/v1/create-qr-code/?size=${size}x${size}&data=${encodeURIComponent(text)}`;
  },

  async downloadQrPng(url, fileName = 'bijios', caption = '') {
      if (!url) return;
      const qrSize = 300;
      const pad = 24;
      const footerH = caption ? 44 : 20;

      const qrCanvas = document.createElement('canvas');
      await this.renderCanvas(qrCanvas, url, qrSize);

      const out = document.createElement('canvas');
      out.width = qrSize + pad * 2;
      out.height = qrSize + pad * 2 + footerH;
      const ctx = out.getContext('2d');

      ctx.fillStyle = '#ffffff';
      ctx.fillRect(0, 0, out.width, out.height);
      ctx.drawImage(qrCanvas, pad, pad);

      if (caption) {
        ctx.fillStyle = '#0f172a';
        ctx.font = '700 13px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif';
        ctx.textAlign = 'center';
        const label = caption.length > 36 ? caption.slice(0, 35) + '…' : caption;
        ctx.fillText(label, out.width / 2, qrSize + pad + 24);
        ctx.fillStyle = '#64748b';
        ctx.font = '500 10px sans-serif';
        ctx.fillText('Scan with phone camera • BijiOS', out.width / 2, qrSize + pad + 40);
      }

      const cleanName = String(fileName || 'bijios').replace(/[^a-zA-Z0-9._-]/g, '_');
      const a = document.createElement('a');
      a.href = out.toDataURL('image/png');
      a.download = `${cleanName}_qr.png`;
      a.click();
    },

    openModal({ title = 'QR Code', fileName = 'dokumen', shareUrl = '', downloadUrl = '', defaultMode = 'share' }) {
      const existing = document.getElementById('dokuhub-qr-modal');
      if (existing) existing.remove();

      const hasBoth = Boolean(shareUrl && downloadUrl);
      let activeMode = (defaultMode === 'download' && downloadUrl) ? 'download' : (shareUrl ? 'share' : 'download');
      const getActiveUrl = () => (activeMode === 'download' ? downloadUrl : shareUrl);

      const overlay = document.createElement('div');
      overlay.id = 'dokuhub-qr-modal';
      overlay.style.cssText = 'position:fixed; inset:0; background:rgba(0,0,0,0.52); backdrop-filter:blur(10px); z-index:999999; display:flex; align-items:center; justify-content:center;';
      overlay.innerHTML = `
      <div style="width:370px; max-width:calc(100vw - 24px); background:var(--window-bg); border:1px solid var(--window-border); border-radius:18px; padding:22px; box-shadow:0 30px 70px rgba(0,0,0,0.4); text-align:center;">
        <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:12px;">
          <h3 style="font-size:15px; font-weight:700; margin:0; text-align:left; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">📱 ${title}</h3>
          <button class="traffic-btn btn-close" onclick="document.getElementById('dokuhub-qr-modal').remove()"></button>
        </div>

        <div style="font-size:12px; color:var(--text-muted); margin-bottom:12px; word-break:break-word;">📄 <strong>${fileName}</strong></div>

        ${hasBoth ? `
          <div style="display:flex; gap:6px; justify-content:center; margin-bottom:12px;">
            <button id="dokuqr-tab-share" class="mac-btn ${activeMode === 'share' ? 'primary' : ''}" style="font-size:11.5px; padding:4px 10px;">🔗 QR Share</button>
            <button id="dokuqr-tab-dl" class="mac-btn ${activeMode === 'download' ? 'primary' : ''}" style="font-size:11.5px; padding:4px 10px;">⬇️ QR Download</button>
          </div>
        ` : ''}

        <div style="display:inline-flex; padding:12px; border-radius:14px; background:#ffffff; border:1px solid rgba(0,0,0,0.08); box-shadow:0 8px 24px rgba(0,0,0,0.1); margin-bottom:12px;">
          <canvas id="dokuqr-modal-canvas" width="200" height="200"></canvas>
        </div>

        <div id="dokuqr-mode-hint" style="font-size:11px; color:var(--text-muted); margin-bottom:10px;"></div>

        <input id="dokuqr-url-input" type="text" readonly style="width:100%; font-size:11px; padding:6px 8px; border-radius:8px; border:1px solid var(--window-border); background:var(--card-bg); color:var(--text-main); margin-bottom:12px; text-align:center;">

        <div style="display:flex; gap:8px; justify-content:center; flex-wrap:wrap;">
          <button id="dokuqr-copy-btn" class="mac-btn" style="font-size:11.5px; padding:6px 12px;">📋 Salin Link</button>
          <button id="dokuqr-dlpng-btn" class="mac-btn primary" style="font-size:11.5px; padding:6px 12px;">⬇️ Unduh QR (PNG)</button>
          ${downloadUrl ? `<a href="${downloadUrl}" download="${fileName}" class="mac-btn" style="font-size:11.5px; padding:6px 12px; text-decoration:none;">📥 Download File</a>` : ''}
        </div>
      </div>
    `;
      document.body.appendChild(overlay);

      const refresh = () => {
        const url = getActiveUrl();
        const canvas = document.getElementById('dokuqr-modal-canvas');
        const input = document.getElementById('dokuqr-url-input');
        const hint = document.getElementById('dokuqr-mode-hint');
        if (input) input.value = url;
        if (hint) {
          hint.textContent = activeMode === 'download'
            ? 'Scan dengan kamera HP untuk langsung mengunduh berkas ini.'
            : 'Scan dengan kamera HP untuk membuka tautan share dokumen ini.';
        }
        if (canvas) this.renderCanvas(canvas, url, 200);

        const tShare = document.getElementById('dokuqr-tab-share');
        const tDl = document.getElementById('dokuqr-tab-dl');
        if (tShare) tShare.classList.toggle('primary', activeMode === 'share');
        if (tDl) tDl.classList.toggle('primary', activeMode === 'download');
      };

      const tShare = document.getElementById('dokuqr-tab-share');
      const tDl = document.getElementById('dokuqr-tab-dl');
      if (tShare) tShare.onclick = () => { activeMode = 'share'; refresh(); };
      if (tDl) tDl.onclick = () => { activeMode = 'download'; refresh(); };

      document.getElementById('dokuqr-copy-btn').onclick = async () => {
        const url = getActiveUrl();
        const ok = await this.copyText(url);
        if (ok) {
          const btn = document.getElementById('dokuqr-copy-btn');
          if (btn) {
            const orig = btn.innerHTML;
            btn.innerHTML = '✅ Tersalin!';
            setTimeout(() => { btn.innerHTML = orig; }, 1600);
          }
          if (window.DokuIsland) DokuIsland.info('Tautan berhasil disalin ke clipboard!', 'QR Share');
        } else {
          const inp = document.getElementById('dokuqr-url-input');
          if (inp) { inp.focus(); inp.select(); }
          alert('Salin manual tautan yang sudah dipilih (Ctrl+C / Cmd+C).');
        }
      };
      document.getElementById('dokuqr-dlpng-btn').onclick = () => {
        const suffix = activeMode === 'download' ? `${fileName} (Download)` : `${fileName} (Share)`;
        this.downloadQrPng(getActiveUrl(), fileName, suffix);
      };

      refresh();
    }
  };
