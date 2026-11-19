/**
 * DokuHub - macOS Preview App (Pratinjau Gambar)
 * Handles .jpg, .jpeg, .png, .bmp, .webp, .gif, .svg
 * Supports zooming, rotating, image inspection, and setting as desktop wallpaper!
 */

window.DokuPreview = {
  containerEl: null,
  currentFile: null,
  zoomLevel: 1.0,
  rotation: 0,
  _currentBlobUrl: null,
  _base64Data: null,

  init(containerEl, file = null) {
    this.containerEl = containerEl;
    this.currentFile = file;
    this.zoomLevel = 1.0;
    this.rotation = 0;
    this._currentBlobUrl = null;
    this._base64Data = null;
    this.render();
    this.updateWindowTitle();
    this.loadImageData(file);
  },

  updateWindowTitle() {
    if (!this.currentFile) return;
    const titleEl = document.querySelector('#window-preview .window-title');
    if (titleEl) {
      titleEl.textContent = `🖼️ ${this.currentFile.name || 'Preview'}`;
    }
    const topAppName = document.getElementById('active-app-name');
    if (topAppName && DokuHub.activeAppId === 'preview') {
      topAppName.textContent = `Preview — ${this.currentFile.name || 'Image'}`;
    }
  },

  async loadImageData(file) {
    if (!file) return;
    const imgEl = document.getElementById('preview-image-el');
    const spinner = document.getElementById('preview-loading-spinner');

    // 1. If file already has base64 data URI, use it directly
    if (file.content && file.content.startsWith('data:')) {
      this._base64Data = file.content;
      if (imgEl) imgEl.src = file.content;
      if (spinner) spinner.style.display = 'none';
      return;
    }

    // 2. Fetch binary with session Bearer token
    try {
      const headers = {};
      const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
      if (token) headers['Authorization'] = `Bearer ${token}`;

      const res = await fetch(`/api/files/${file.id}/download`, { headers });
      if (res.ok) {
        const blob = await res.blob();
        if (this._currentBlobUrl) {
          URL.revokeObjectURL(this._currentBlobUrl);
        }
        const blobUrl = URL.createObjectURL(blob);
        this._currentBlobUrl = blobUrl;

        // Also convert to base64 for persistent wallpaper use
        const reader = new FileReader();
        reader.onloadend = () => {
          this._base64Data = reader.result;
        };
        reader.readAsDataURL(blob);

        if (imgEl) {
          imgEl.src = blobUrl;
        }
        if (spinner) spinner.style.display = 'none';
        return;
      }
    } catch (e) {
      console.warn("Could not fetch image blob via Bearer auth:", e);
    }

    // 3. Fallback direct download URL (public stream)
    if (imgEl && !imgEl.src) {
      imgEl.src = `/api/files/${file.id}/download`;
    }
    if (spinner) spinner.style.display = 'none';
  },

  getImageSrc() {
    if (!this.currentFile) return '';
    const f = this.currentFile;
    if (this._currentBlobUrl) return this._currentBlobUrl;
    if (this._base64Data) return this._base64Data;
    if (f.content && (f.content.startsWith('data:') || f.content.startsWith('http://') || f.content.startsWith('https://'))) {
      return f.content;
    }
    return `/api/files/${f.id}/download`;
  },

  zoomIn() {
    this.zoomLevel = Math.min(4.0, this.zoomLevel + 0.25);
    this.applyTransform();
  },

  zoomOut() {
    this.zoomLevel = Math.max(0.25, this.zoomLevel - 0.25);
    this.applyTransform();
  },

  resetZoom() {
    this.zoomLevel = 1.0;
    this.rotation = 0;
    this.applyTransform();
  },

  rotate() {
    this.rotation = (this.rotation + 90) % 360;
    this.applyTransform();
  },

  applyTransform() {
    const img = document.getElementById('preview-image-el');
    const zoomText = document.getElementById('preview-zoom-label');
    if (img) {
      img.style.transform = `scale(${this.zoomLevel}) rotate(${this.rotation}deg)`;
    }
    if (zoomText) {
      zoomText.textContent = `${Math.round(this.zoomLevel * 100)}%`;
    }
  },

  async setAsWallpaper() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const f = this.currentFile;
    if (!f) return;

    let targetSrc = this._base64Data || this.getImageSrc();

    if (!targetSrc || targetSrc.startsWith('blob:')) {
      // Convert to persistent base64
      try {
        const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
        const headers = token ? { 'Authorization': `Bearer ${token}` } : {};
        const res = await fetch(`/api/files/${f.id}/download`, { headers });
        if (res.ok) {
          const blob = await res.blob();
          const reader = new FileReader();
          reader.onloadend = () => {
            const dataUrl = reader.result;
            this._base64Data = dataUrl;
            DokuHub.setWallpaper(dataUrl, true);
            alert(isId 
              ? `✅ "${f.name}" berhasil dijadikan wallpaper desktop!`
              : `✅ "${f.name}" set as desktop wallpaper!`);
          };
          reader.readAsDataURL(blob);
          return;
        }
      } catch (e) {
        console.warn("Failed to convert image for wallpaper:", e);
      }
    }

    if (targetSrc) {
      DokuHub.setWallpaper(targetSrc, true);
      alert(isId 
        ? `✅ "${f.name}" berhasil dijadikan wallpaper desktop!`
        : `✅ "${f.name}" set as desktop wallpaper!`);
    }
  },

  render() {
    if (!this.containerEl) return;
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const f = this.currentFile || { name: 'Gambar.jpg', extension: 'jpg', size_bytes: 1250000 };
    const initialSrc = (f.content && f.content.startsWith('data:')) ? f.content : `/api/files/${f.id}/download`;

    this.containerEl.innerHTML = `
      <div class="preview-app-container">
        <!-- Top macOS Toolbar -->
        <div class="preview-toolbar">
          <div class="preview-toolbar-left">
            <span class="preview-file-badge">${(f.extension || 'IMG').toUpperCase()}</span>
            <span class="preview-file-name" title="${f.name}">${f.name}</span>
          </div>

          <div class="preview-toolbar-center">
            <button class="mac-btn preview-tool-btn" onclick="DokuPreview.zoomOut()" title="${isId ? 'Perkecil' : 'Zoom Out'}">🔍−</button>
            <span class="preview-zoom-level" id="preview-zoom-label">100%</span>
            <button class="mac-btn preview-tool-btn" onclick="DokuPreview.zoomIn()" title="${isId ? 'Perbesar' : 'Zoom In'}">🔍+</button>
            <button class="mac-btn preview-tool-btn" onclick="DokuPreview.resetZoom()" title="${isId ? 'Ukuran Normal' : 'Actual Size'}">1:1</button>
            <button class="mac-btn preview-tool-btn" onclick="DokuPreview.rotate()" title="${isId ? 'Putar 90 Derajat' : 'Rotate 90°'}">⟳</button>
          </div>

          <div class="preview-toolbar-right">
            <button class="mac-btn primary preview-btn-wallpaper" onclick="DokuPreview.setAsWallpaper()" title="${isId ? 'Gunakan gambar ini sebagai wallpaper latar belakang' : 'Set as Desktop Wallpaper'}">
              🌄 ${isId ? 'Jadikan Wallpaper' : 'Set as Wallpaper'}
            </button>
            <a href="/api/files/${f.id}/download" target="_blank" download="${f.name}" class="mac-btn preview-tool-btn" style="text-decoration:none; display:inline-flex; align-items:center;" title="${isId ? 'Unduh Berkas Asli' : 'Download File'}">
              ⬇️
            </a>
          </div>
        </div>

        <!-- Main Image Canvas Viewport -->
        <div class="preview-canvas-viewport" id="preview-canvas-viewport">
          <div id="preview-loading-spinner" style="position:absolute; display:flex; align-items:center; justify-content:center; gap:8px; color:var(--text-muted); font-size:13px; z-index:5;">
            <div style="width:16px; height:16px; border:2px solid var(--text-muted); border-top-color:transparent; border-radius:50%; animation:spin 0.8s linear infinite;"></div>
            <span>${isId ? 'Memuat gambar...' : 'Loading image...'}</span>
          </div>
          <div class="preview-canvas-wrapper">
            <img id="preview-image-el" class="preview-image-element" src="${initialSrc}" alt="${f.name}"
              onload="document.getElementById('preview-loading-spinner')?.remove();"
              onerror="document.getElementById('preview-loading-spinner')?.remove(); this.style.display='none'; document.getElementById('preview-error-box').style.display='flex';">
            <div id="preview-error-box" style="display:none; flex-direction:column; align-items:center; justify-content:center; text-align:center; padding:30px; color:var(--text-muted);">
              <div style="font-size:44px; margin-bottom:12px;">🖼️</div>
              <div style="font-size:14px; font-weight:600; color:var(--text-main); margin-bottom:4px;">${isId ? 'Gagal memuat pratinjau gambar' : 'Failed to load image preview'}</div>
              <div style="font-size:12px; max-width:320px;">${isId ? 'Pastikan berkas gambar valid atau coba unduh berkas menggunakan tombol unduh di atas.' : 'Please ensure the image file is valid or try downloading using the button above.'}</div>
            </div>
          </div>
        </div>

        <!-- Footer Meta Bar -->
        <div class="preview-statusbar">
          <div>
            <span>${isId ? 'Tipe' : 'Kind'}: ${f.mime_type || ('image/' + f.extension)}</span>
            <span class="meta-separator">•</span>
            <span>${isId ? 'Ukuran' : 'Size'}: ${DokuFinder ? DokuFinder.formatSize(f.size_bytes || 1200000) : '1.2 MB'}</span>
          </div>
          <div style="color:var(--text-muted); font-size:11px;">
            ${isId ? 'Klik "Jadikan Wallpaper" untuk menyesuaikan desktop Anda' : 'Click "Set as Wallpaper" to personalize your desktop'}
          </div>
        </div>
      </div>
    `;
  }
};
