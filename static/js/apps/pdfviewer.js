/**
 * DokuPdfViewer (PDFCraft) - Adapted from storytold/pdfcraft for DokuHub
 * Full-featured PDF Reader & Non-Destructive Editor powered by PDF.js + pdf-lib:
 * - High-DPI PDF rendering & live thumbnail page navigator
 * - Non-destructive Markup & Editing: Text, Highlight, Freehand Pen/Signature, Box, Sticky Notes, Stamps
 * - Full Undo/Redo history & normalized coordinate precision
 * - Page Organizer: Add Blank Page, Rotate 90°, Reorder Up/Down, Delete, Merge PDF, Split/Extract Page
 * - Full-text search across pages
 * - Atomic save back to DokuHub Finder & standard .pdf download
 */

window.DokuPdfViewer = {
  container: null,
  currentFile: null,      // { id?, name, dataUrl?, parent_id? }
  pdfBytes: null,         // Uint8Array of current PDF document
  pdfJsDoc: null,         // PDF.js document proxy
  currentPage: 1,
  totalPages: 0,
  zoom: 1.15,             // 1.0 = 100%
  activeTool: 'select',   // select | text | highlight | pen | rect | note | stamp
  activeColor: '#ef4444',
  activeSize: 16,
  activeStamp: 'APPROVED',
  showThumbnails: true,
  showInspector: true,
  searchQuery: '',
  searchResults: [],      // [{ page, snippet }]

  // Non-destructive annotations stored per 1-based page number with normalized [0..1] coords
  annotations: {},        // { [pageNum]: [ { id, type, x, y, w, h, text, color, size, points } ] }
  selectedAnnotId: null,
  undoStack: [],
  redoStack: [],
  hasUnsavedChanges: false,

  // Drawing interaction state
  _isDrawing: false,
  _dragStart: null,
  _currentDraft: null,
  _availablePdfs: [],
  _renderTask: null,

  init(containerEl, file = null) {
    this.container = containerEl;
    this.currentFile = file;
    this.pdfBytes = null;
    this.pdfJsDoc = null;
    this.currentPage = 1;
    this.totalPages = 0;
    this.zoom = window.innerWidth <= 768 ? 0.75 : 1.1;
    this.activeTool = 'select';
    this.annotations = {};
    this.selectedAnnotId = null;
    this.undoStack = [];
    this.redoStack = [];
    this.hasUnsavedChanges = false;
    this.searchQuery = '';
    this.searchResults = [];
    this.showThumbnails = window.innerWidth > 768;
    this.showInspector = window.innerWidth > 1080;

    if (window.pdfjsLib) {
      pdfjsLib.GlobalWorkerOptions.workerSrc =
        'https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/pdf.worker.min.js';
    }

    this.render();
    this.updateWindowTitle();

    if (file) {
      this.loadFile(file);
    }
  },

  updateWindowTitle() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const dirty = this.hasUnsavedChanges ? ' •' : '';
    const docName = this.currentFile
      ? `${this.currentFile.name}${dirty}`
      : (isId ? 'Tidak Ada Berkas' : 'No Document Open');

    const win = document.getElementById('window-pdfviewer');
    if (win) {
      const titleEl = win.querySelector('.window-title');
      if (titleEl) titleEl.textContent = `📄 PDFCraft — ${docName}`;
    }
    const appTitleMenu = document.getElementById('active-app-name');
    if (appTitleMenu && window.DokuHub && DokuHub.activeAppId === 'pdfviewer') {
      appTitleMenu.textContent = `PDFCraft — ${docName}`;
    }
  },

  getAuthHeaders() {
    const headers = {};
    const token = sessionStorage.getItem('dokuhub_token') || localStorage.getItem('dokuhub_token');
    if (token) headers['Authorization'] = `Bearer ${token}`;
    if (window.DokuHub && DokuHub.activeTenant && DokuHub.activeTenant.id) {
      headers['X-Tenant-Id'] = DokuHub.activeTenant.id;
    }
    if (window.DokuHub && DokuHub.currentUser && DokuHub.currentUser.id) {
      headers['X-User-Id'] = DokuHub.currentUser.id;
    }
    return headers;
  },

  async ensureEnginesReady() {
    let tries = 0;
    while ((!window.pdfjsLib || !window.PDFLib) && tries < 40) {
      await new Promise((r) => setTimeout(r, 150));
      tries++;
    }
    if (window.pdfjsLib) {
      pdfjsLib.GlobalWorkerOptions.workerSrc =
        'https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/pdf.worker.min.js';
    }
    if (!window.pdfjsLib || !window.PDFLib) {
      throw new Error('Engine PDF.js / pdf-lib gagal dimuat. Pastikan koneksi internet aktif.');
    }
  },

  /* ------------------------------------------------------------------------
     Document Loading & Creation
     ------------------------------------------------------------------------ */
  async createBlankPdf() {
    try {
      await this.ensureEnginesReady();
      const doc = await PDFLib.PDFDocument.create();
      doc.addPage([595.28, 841.89]); // Standard A4
      const bytes = await doc.save();

      this.currentFile = { id: null, name: 'Dokumen Baru.pdf', isLocal: true };
      this.annotations = {};
      this.undoStack = [];
      this.redoStack = [];
      this.hasUnsavedChanges = true;
      await this.loadPdfBytes(bytes, 1);

      if (window.DokuIsland) {
        DokuIsland.info('Dokumen PDF A4 baru siap diedit', 'PDFCraft');
      }
    } catch (e) {
      alert('Gagal membuat PDF baru: ' + e.message);
    }
  },

  async loadFile(file) {
    if (!file) return;
    this.currentFile = file;
    this.annotations = {};
    this.undoStack = [];
    this.redoStack = [];
    this.selectedAnnotId = null;
    this.hasUnsavedChanges = false;
    this.render();
    this.updateWindowTitle();

    try {
      await this.ensureEnginesReady();
      let bytes;

      if (file.bytes instanceof Uint8Array) {
        bytes = file.bytes;
      } else if (file.dataUrl && file.dataUrl.startsWith('data:')) {
        const b64 = file.dataUrl.split(',')[1] || '';
        const bin = atob(b64);
        bytes = new Uint8Array(bin.length);
        for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
      } else if (file.id) {
        const res = await fetch(`/api/files/${file.id}/download`, { headers: this.getAuthHeaders() });
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        const buf = await res.arrayBuffer();
        bytes = new Uint8Array(buf);
      } else {
        throw new Error('Sumber berkas PDF tidak valid.');
      }

      await this.loadPdfBytes(bytes, 1);
      if (window.DokuIsland) {
        DokuIsland.info(`Membuka "${file.name}"`, 'PDFCraft');
      }
    } catch (e) {
      console.error('PDFCraft load error:', e);
      alert('Gagal memuat dokumen PDF: ' + e.message);
    }
  },

  async loadPdfBytes(bytes, targetPage = 1) {
    await this.ensureEnginesReady();
    // Always keep a clean copy because PDF.js may detach the passed buffer
    this.pdfBytes = new Uint8Array(bytes);

    // Detect legacy corrupted PDF uploads (UTF-8 U+FFFD replacement bytes EF BF BD in PDF header/body)
    let looksCorrupted = false;
    for (let i = 0; i < Math.min(this.pdfBytes.length - 2, 128); i++) {
      if (this.pdfBytes[i] === 0xef && this.pdfBytes[i + 1] === 0xbf && this.pdfBytes[i + 2] === 0xbd) {
        looksCorrupted = true;
        break;
      }
    }
    this._corruptedWarning = looksCorrupted;

    const loadingTask = pdfjsLib.getDocument({
      data: new Uint8Array(this.pdfBytes),
      cMapUrl: 'https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/cmaps/',
      cMapPacked: true,
      standardFontDataUrl: 'https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/standard_fonts/',
      useSystemFonts: true
    });
    this.pdfJsDoc = await loadingTask.promise;
    this.totalPages = this.pdfJsDoc.numPages;
    this.currentPage = Math.max(1, Math.min(targetPage, this.totalPages));
    this.render();
    this.updateWindowTitle();
    await this.renderActivePage();
    if (this.showThumbnails) {
      this.renderThumbnails();
    }
    if (looksCorrupted && window.DokuIsland) {
      DokuIsland.info('File PDF lama ini rusak saat upload sebelumnya. Hapus & upload ulang di Finder agar tampil normal.', 'PDFCraft');
    }
  },

  handleLocalUpload(event) {
    const file = event.target.files[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = (e) => {
      const bytes = new Uint8Array(e.target.result);
      this.loadFile({
        id: null,
        name: file.name,
        bytes,
        isLocal: true
      });
    };
    reader.readAsArrayBuffer(file);
    event.target.value = '';
  },

  closeDocument() {
    if (this.hasUnsavedChanges && !confirm('Ada perubahan belum disimpan. Tetap tutup dokumen?')) return;
    this.currentFile = null;
    this.pdfBytes = null;
    this.pdfJsDoc = null;
    this.totalPages = 0;
    this.currentPage = 1;
    this.annotations = {};
    this.hasUnsavedChanges = false;
    this.render();
    this.updateWindowTitle();
  },

  /* ------------------------------------------------------------------------
     Main UI Renderer
     ------------------------------------------------------------------------ */
  render() {
    if (!this.container) return;
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');

    // 1. Welcome / Start Screen
    if (!this.currentFile) {
      this.container.innerHTML = `
        <div style="display:flex; flex-direction:column; height:100%; background:var(--window-bg); align-items:center; justify-content:center; padding:24px; position:relative; overflow:hidden;">
          <div style="position:absolute; width:380px; height:380px; border-radius:50%; background:radial-gradient(circle, rgba(239,68,68,0.14), transparent 70%); filter:blur(42px); pointer-events:none;"></div>

          <div class="mac-card" style="width:100%; max-width:520px; padding:34px 28px; text-align:center; background:var(--card-bg); backdrop-filter:blur(24px); border:1px solid var(--window-border); border-radius:18px; box-shadow:0 24px 60px rgba(0,0,0,0.18); position:relative; z-index:1;">
            <div style="width:72px; height:72px; border-radius:18px; background:linear-gradient(135deg, #ef4444, #b91c1c); color:#fff; display:flex; align-items:center; justify-content:center; font-size:34px; margin:0 auto 16px; box-shadow:0 8px 24px rgba(239,68,68,0.35);">
              📄
            </div>
            <h2 style="font-size:21px; font-weight:800; margin:0 0 6px; color:var(--text-main);">PDFCraft Studio</h2>
            <p style="font-size:12.5px; color:var(--text-muted); line-height:1.5; margin:0 0 22px;">
              ${isId
                ? 'Baca, edit teks, beri highlight, tanda tangan, catatan, stempel, atur halaman, gabung (Merge) & pisah (Split) dokumen PDF secara non-destruktif.'
                : 'Read, edit text, highlight, sign, add sticky notes & stamps, organize pages, merge & split PDFs non-destructively.'}
            </p>

            <div style="display:flex; gap:10px; justify-content:center; flex-wrap:wrap;">
              <button class="mac-btn primary" onclick="DokuPdfViewer.createBlankPdf()" style="height:35px; padding:0 15px; font-size:12.5px; font-weight:700;">
                ➕ ${isId ? 'Buat PDF Baru (A4)' : 'New Blank PDF'}
              </button>
              <button class="mac-btn" onclick="DokuPdfViewer.openFromFileManagerModal()" style="height:35px; padding:0 15px; font-size:12.5px; font-weight:600;">
                📂 ${isId ? 'Buka dari Finder...' : 'Open from Finder...'}
              </button>
              <label class="mac-btn" style="height:35px; padding:0 15px; font-size:12.5px; font-weight:600; display:inline-flex; align-items:center; cursor:pointer;">
                📥 ${isId ? 'Unggah PDF Lokal...' : 'Open Local PDF...'}
                <input type="file" accept=".pdf,application/pdf" style="display:none;" onchange="DokuPdfViewer.handleLocalUpload(event)">
              </label>
            </div>
          </div>
        </div>
      `;
      return;
    }

    const pageAnnots = this.annotations[this.currentPage] || [];
    const tools = [
      { id: 'select', icon: '🖱️', label: 'Select' },
      { id: 'text', icon: '🔤', label: 'Text' },
      { id: 'highlight', icon: '🖍️', label: 'Highlight' },
      { id: 'pen', icon: '✒️', label: 'Sign/Pen' },
      { id: 'rect', icon: '▭', label: 'Box' },
      { id: 'note', icon: '💬', label: 'Note' },
      { id: 'stamp', icon: '🔖', label: 'Stamp' }
    ];
    const colors = ['#ef4444', '#2563eb', '#10b981', '#f59e0b', '#7c3aed', '#111827'];

    this.container.innerHTML = `
      <div style="display:flex; flex-direction:column; height:100%; background:var(--window-bg); color:var(--text-main);">
        <!-- Top Primary Toolbar -->
        <div style="min-height:42px; background:var(--card-bg); border-bottom:1px solid var(--window-border); display:flex; align-items:center; justify-content:space-between; padding:4px 10px; gap:8px; flex-wrap:wrap; flex-shrink:0;">
          <div style="display:flex; align-items:center; gap:5px; flex-wrap:wrap;">
            <button class="mac-btn" onclick="DokuPdfViewer.showThumbnails=!DokuPdfViewer.showThumbnails; DokuPdfViewer.render(); DokuPdfViewer.renderActivePage(); if(DokuPdfViewer.showThumbnails) DokuPdfViewer.renderThumbnails();" style="height:27px; padding:0 8px; font-size:12px;" title="Toggle Thumbnails">◧</button>
            <button class="mac-btn" onclick="DokuPdfViewer.openFromFileManagerModal()" style="height:27px; padding:0 8px; font-size:11.5px;">📂 ${isId ? 'Buka' : 'Open'}</button>
            <button class="mac-btn" onclick="DokuPdfViewer.createBlankPdf()" style="height:27px; padding:0 8px; font-size:11.5px;">➕ ${isId ? 'Baru' : 'New'}</button>
            <label class="mac-btn" style="height:27px; padding:0 8px; font-size:11.5px; display:inline-flex; align-items:center; cursor:pointer;" title="${isId ? 'Gabungkan (Merge) PDF lain' : 'Merge another PDF'}">
              🔗 Merge
              <input type="file" accept=".pdf,application/pdf" style="display:none;" onchange="DokuPdfViewer.mergeLocalPdf(event)">
            </label>
            <button class="mac-btn" onclick="DokuPdfViewer.extractCurrentPage()" style="height:27px; padding:0 8px; font-size:11.5px;" title="${isId ? 'Ekstrak halaman aktif jadi PDF terpisah' : 'Extract current page'}">✂️ Split</button>
          </div>

          <!-- Center Page & Zoom Controls -->
          <div style="display:flex; align-items:center; gap:5px;">
            <button class="mac-btn" onclick="DokuPdfViewer.goToPage(DokuPdfViewer.currentPage - 1)" style="height:27px; padding:0 7px; font-size:11px;">◀</button>
            <span style="font-size:11.5px; font-weight:700; min-width:58px; text-align:center;">${this.currentPage} / ${this.totalPages || 1}</span>
            <button class="mac-btn" onclick="DokuPdfViewer.goToPage(DokuPdfViewer.currentPage + 1)" style="height:27px; padding:0 7px; font-size:11px;">▶</button>
            <div style="width:1px; height:16px; background:var(--window-border); margin:0 3px;"></div>
            <button class="mac-btn" onclick="DokuPdfViewer.setZoom(DokuPdfViewer.zoom - 0.15)" style="height:27px; padding:0 7px; font-size:12px;">−</button>
            <span style="font-size:11px; font-weight:700; min-width:42px; text-align:center;">${Math.round(this.zoom * 100)}%</span>
            <button class="mac-btn" onclick="DokuPdfViewer.setZoom(DokuPdfViewer.zoom + 0.15)" style="height:27px; padding:0 7px; font-size:12px;">+</button>
            <button class="mac-btn" onclick="DokuPdfViewer.rotateCurrentPage()" style="height:27px; padding:0 7px; font-size:12px;" title="${isId ? 'Putar Halaman 90°' : 'Rotate Page 90°'}">↻</button>
          </div>

          <!-- Right Save & Inspector -->
          <div style="display:flex; align-items:center; gap:5px; flex-wrap:wrap;">
            <button class="mac-btn" onclick="DokuPdfViewer.undo()" style="height:27px; padding:0 7px; font-size:12px;" title="Undo" ${this.undoStack.length ? '' : 'disabled'}>↩️</button>
            <button class="mac-btn" onclick="DokuPdfViewer.redo()" style="height:27px; padding:0 7px; font-size:12px;" title="Redo" ${this.redoStack.length ? '' : 'disabled'}>↪️</button>
            <button class="mac-btn primary" onclick="DokuPdfViewer.saveToFinder()" style="height:27px; padding:0 10px; font-size:11.5px; font-weight:700;">💾 ${isId ? 'Simpan' : 'Save'}</button>
            <button class="mac-btn" onclick="DokuPdfViewer.downloadEditedPdf()" style="height:27px; padding:0 9px; font-size:11.5px;">⬇️ PDF</button>
            <button class="mac-btn" onclick="DokuPdfViewer.showInspector=!DokuPdfViewer.showInspector; DokuPdfViewer.render(); DokuPdfViewer.renderActivePage();" style="height:27px; padding:0 8px; font-size:12px;" title="Inspector & Comments">💬</button>
          </div>
        </div>

        <!-- Secondary Markup & Edit Ribbon -->
        <div style="min-height:38px; background:var(--window-bg); border-bottom:1px solid var(--window-border); display:flex; align-items:center; justify-content:space-between; padding:4px 10px; gap:8px; flex-wrap:wrap; flex-shrink:0;">
          <div style="display:flex; align-items:center; gap:4px; flex-wrap:wrap;">
            ${tools.map(t => `
              <button class="mac-btn ${this.activeTool === t.id ? 'primary' : ''}"
                onclick="DokuPdfViewer.setTool('${t.id}')"
                style="height:26px; padding:0 8px; font-size:11px; font-weight:600;">
                ${t.icon} ${t.label}
              </button>
            `).join('')}

            ${this.activeTool === 'stamp' ? `
              <select onchange="DokuPdfViewer.activeStamp=this.value" style="height:26px; font-size:11px; padding:0 6px; border-radius:6px; border:1px solid var(--window-border); background:var(--card-bg); color:var(--text-main);">
                ${['APPROVED', 'CONFIDENTIAL', 'DRAFT', 'REVIEWED', 'REJECTED'].map(st => `<option value="${st}" ${this.activeStamp === st ? 'selected' : ''}>${st}</option>`).join('')}
              </select>
            ` : ''}
          </div>

          <!-- Colors, Size & Search -->
          <div style="display:flex; align-items:center; gap:6px; flex-wrap:wrap;">
            <div style="display:flex; align-items:center; gap:4px;">
              ${colors.map(c => `
                <button onclick="DokuPdfViewer.activeColor='${c}'; DokuPdfViewer.render(); DokuPdfViewer.renderActivePage();"
                  style="width:16px; height:16px; border-radius:50%; background:${c}; border:${this.activeColor === c ? '2px solid var(--text-main)' : '1px solid rgba(128,128,128,0.4)'}; cursor:pointer; padding:0;"></button>
              `).join('')}
            </div>
            <select onchange="DokuPdfViewer.activeSize=parseInt(this.value)" title="Font / Stroke Size" style="height:25px; font-size:11px; padding:0 5px; border-radius:6px; border:1px solid var(--window-border); background:var(--card-bg); color:var(--text-main);">
              ${[10, 12, 14, 16, 20, 26, 32].map(sz => `<option value="${sz}" ${this.activeSize === sz ? 'selected' : ''}>${sz}px</option>`).join('')}
            </select>
            <div style="display:flex; align-items:center; gap:4px;">
              <input id="pdfcraft-search-input" type="text" value="${(this.searchQuery || '').replace(/"/g, '&quot;')}"
                placeholder="${isId ? 'Cari teks...' : 'Search text...'}"
                onkeydown="if(event.key==='Enter') DokuPdfViewer.searchText(this.value)"
                style="height:25px; width:120px; padding:0 8px; border-radius:6px; border:1px solid var(--window-border); background:var(--card-bg); color:var(--text-main); font-size:11px;">
              <button class="mac-btn" onclick="DokuPdfViewer.searchText(document.getElementById('pdfcraft-search-input').value)" style="height:25px; padding:0 7px; font-size:11px;">🔍</button>
            </div>
          </div>
        </div>

        <!-- 3-Pane Workspace Body -->
        <div style="flex:1; display:flex; min-height:0; overflow:hidden;">
          <!-- Left Sidebar: Thumbnails & Page Organizer -->
          ${this.showThumbnails ? `
            <div style="width:176px; background:var(--card-bg); border-right:1px solid var(--window-border); display:flex; flex-direction:column; flex-shrink:0;">
              <div style="padding:8px; border-bottom:1px solid var(--window-border); display:flex; justify-content:space-between; align-items:center;">
                <span style="font-size:11px; font-weight:700; color:var(--text-muted);">HALAMAN (${this.totalPages})</span>
                <button class="mac-btn" onclick="DokuPdfViewer.addBlankPage()" style="height:22px; padding:0 6px; font-size:10.5px;" title="Tambah Halaman Kosong">+ Page</button>
              </div>
              <div id="pdfcraft-thumbs" style="flex:1; overflow-y:auto; padding:8px; display:flex; flex-direction:column; gap:10px;">
                <div style="font-size:11px; color:var(--text-muted); text-align:center; padding:12px;">Memuat halaman...</div>
              </div>
              <div style="padding:6px 8px; border-top:1px solid var(--window-border); display:flex; justify-content:space-between; gap:4px;">
                <button class="mac-btn" onclick="DokuPdfViewer.moveCurrentPage(-1)" style="flex:1; height:24px; font-size:11px;" title="Pindah ke Atas">↑</button>
                <button class="mac-btn" onclick="DokuPdfViewer.moveCurrentPage(1)" style="flex:1; height:24px; font-size:11px;" title="Pindah ke Bawah">↓</button>
                <button class="mac-btn" onclick="DokuPdfViewer.deleteCurrentPage()" style="flex:1; height:24px; font-size:11px; color:#ef4444;" title="Hapus Halaman">🗑️</button>
              </div>
            </div>
          ` : ''}

          <!-- Center PDF Canvas Viewport -->
          <div id="pdfcraft-viewport" style="flex:1; overflow:auto; background:rgba(15,23,42,0.12); display:flex; flex-direction:column; align-items:center; padding:20px; position:relative;">
            <div id="pdfcraft-stage" style="position:relative; box-shadow:0 12px 36px rgba(0,0,0,0.28); background:#fff; line-height:0;">
              <canvas id="pdfcraft-render-canvas" style="display:block;"></canvas>
              <canvas id="pdfcraft-annot-canvas" style="position:absolute; inset:0; width:100%; height:100%; cursor:${this.activeTool === 'select' ? 'default' : 'crosshair'}; touch-action:none;"></canvas>
            </div>
          </div>

          <!-- Right Inspector: Annotations, Sticky Notes & Search Results -->
          ${this.showInspector ? `
            <div style="width:230px; background:var(--card-bg); border-left:1px solid var(--window-border); display:flex; flex-direction:column; flex-shrink:0; overflow-y:auto; padding:10px; gap:12px;">
              ${this.searchResults.length > 0 ? `
                <div>
                  <div style="font-size:11px; font-weight:700; color:var(--accent-color); margin-bottom:6px;">🔍 HASIL PENCARIAN (${this.searchResults.length})</div>
                  <div style="display:flex; flex-direction:column; gap:5px; max-height:140px; overflow-y:auto;">
                    ${this.searchResults.map(r => `
                      <div onclick="DokuPdfViewer.goToPage(${r.page})" style="padding:6px 8px; border-radius:7px; background:var(--window-bg); border:1px solid var(--window-border); cursor:pointer; font-size:11px;">
                        <div style="font-weight:700;">Halaman ${r.page}</div>
                        <div style="color:var(--text-muted); white-space:nowrap; overflow:hidden; text-overflow:ellipsis;">${r.snippet}</div>
                      </div>
                    `).join('')}
                  </div>
                </div>
              ` : ''}

              <div>
                <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:6px;">
                  <span style="font-size:11px; font-weight:700; color:var(--text-muted);">ANOTASI HAL. ${this.currentPage} (${pageAnnots.length})</span>
                  ${pageAnnots.length ? `<button class="mac-btn" onclick="DokuPdfViewer.clearPageAnnotations()" style="height:20px; padding:0 6px; font-size:10px; color:#ef4444;">Hapus</button>` : ''}
                </div>

                <div style="display:flex; flex-direction:column; gap:6px;">
                  ${pageAnnots.length === 0 ? `
                    <div style="font-size:11px; color:var(--text-muted); padding:10px; text-align:center; border:1px dashed var(--window-border); border-radius:8px;">
                      Pilih alat di atas (Text, Highlight, Sign/Pen, Box, Note, Stamp) lalu klik/seret pada halaman PDF.
                    </div>
                  ` : pageAnnots.map(a => `
                    <div onclick="DokuPdfViewer.selectedAnnotId='${a.id}'; DokuPdfViewer.render(); DokuPdfViewer.renderActivePage();"
                      style="padding:7px 8px; border-radius:8px; background:var(--window-bg); border:1px solid ${this.selectedAnnotId === a.id ? 'var(--accent-color)' : 'var(--window-border)'}; font-size:11px; cursor:pointer;">
                      <div style="display:flex; justify-content:space-between; align-items:center;">
                        <span style="font-weight:700; color:${a.color || 'var(--text-main)'}; text-transform:uppercase;">${a.type}</span>
                        <button onclick="event.stopPropagation(); DokuPdfViewer.deleteAnnotation('${a.id}')" style="background:none; border:none; color:#ef4444; cursor:pointer; font-size:11px;">✕</button>
                      </div>
                      ${a.text ? `<div style="margin-top:3px; color:var(--text-main); word-break:break-word;">${a.text}</div>` : ''}
                    </div>
                  `).join('')}
                </div>
              </div>
            </div>
          ` : ''}
        </div>
      </div>
    `;

    this.bindAnnotationEvents();
  },

  setTool(toolId) {
    this.activeTool = toolId;
    this.selectedAnnotId = null;
    this.render();
    this.renderActivePage();
  },

  setZoom(nextZoom) {
    this.zoom = Math.max(0.45, Math.min(2.6, Number(nextZoom.toFixed(2))));
    this.render();
    this.renderActivePage();
  },

  async goToPage(pageNum) {
    if (!this.pdfJsDoc) return;
    const clamped = Math.max(1, Math.min(pageNum, this.totalPages));
    if (clamped === this.currentPage) return;
    this.currentPage = clamped;
    this.selectedAnnotId = null;
    this.render();
    await this.renderActivePage();
    if (this.showThumbnails) this.renderThumbnails();
  },

  /* ------------------------------------------------------------------------
     PDF.js Rendering (Main Page & Thumbnails)
     ------------------------------------------------------------------------ */
  async renderActivePage() {
    if (!this.pdfJsDoc) return;
    const renderCanvas = document.getElementById('pdfcraft-render-canvas');
    const annotCanvas = document.getElementById('pdfcraft-annot-canvas');
    if (!renderCanvas || !annotCanvas) return;

    try {
      if (this._renderTask) {
        try { this._renderTask.cancel(); } catch {}
      }
      const page = await this.pdfJsDoc.getPage(this.currentPage);
      const viewport = page.getViewport({ scale: this.zoom });
      const dpr = Math.min(window.devicePixelRatio || 1, 2);

      renderCanvas.width = Math.floor(viewport.width * dpr);
      renderCanvas.height = Math.floor(viewport.height * dpr);
      renderCanvas.style.width = `${Math.floor(viewport.width)}px`;
      renderCanvas.style.height = `${Math.floor(viewport.height)}px`;

      annotCanvas.width = Math.floor(viewport.width * dpr);
      annotCanvas.height = Math.floor(viewport.height * dpr);
      annotCanvas.style.width = `${Math.floor(viewport.width)}px`;
      annotCanvas.style.height = `${Math.floor(viewport.height)}px`;

      const ctx = renderCanvas.getContext('2d');
      ctx.setTransform(1, 0, 0, 1, 0, 0);
      ctx.fillStyle = '#ffffff';
      ctx.fillRect(0, 0, renderCanvas.width, renderCanvas.height);

      const transform = dpr !== 1 ? [dpr, 0, 0, dpr, 0, 0] : null;
      this._renderTask = page.render({ canvasContext: ctx, viewport, transform });
      await this._renderTask.promise;
      this._renderTask = null;

      this.redrawAnnotations();
    } catch (e) {
      if (e && e.name !== 'RenderingCancelledException') {
        console.warn('PDF render error:', e);
      }
    }
  },

  async renderThumbnails() {
    if (!this.pdfJsDoc) return;
    const wrap = document.getElementById('pdfcraft-thumbs');
    if (!wrap) return;
    wrap.innerHTML = '';

    for (let p = 1; p <= this.totalPages; p++) {
      const card = document.createElement('div');
      card.style.cssText = `padding:6px; border-radius:8px; cursor:pointer; text-align:center; border:2px solid ${p === this.currentPage ? 'var(--accent-color, #007aff)' : 'var(--window-border)'}; background:${p === this.currentPage ? 'rgba(0,122,255,0.08)' : 'var(--window-bg)'};`;
      card.onclick = () => this.goToPage(p);

      const canvas = document.createElement('canvas');
      canvas.style.cssText = 'width:100%; height:auto; display:block; border-radius:4px; background:#fff; box-shadow:0 2px 6px rgba(0,0,0,0.12);';
      const label = document.createElement('div');
      label.style.cssText = 'font-size:10.5px; font-weight:700; margin-top:4px; color:var(--text-muted);';
      label.textContent = `Hal. ${p}`;

      card.appendChild(canvas);
      card.appendChild(label);
      wrap.appendChild(card);

      try {
        const page = await this.pdfJsDoc.getPage(p);
        const vp = page.getViewport({ scale: 0.26 });
        canvas.width = Math.floor(vp.width);
        canvas.height = Math.floor(vp.height);
        await page.render({ canvasContext: canvas.getContext('2d'), viewport: vp }).promise;
      } catch {}
    }
  },

  /* ------------------------------------------------------------------------
     Interactive Annotation Overlay (Mouse & Touch)
     ------------------------------------------------------------------------ */
  getNormalizedPoint(evt, canvas) {
    const rect = canvas.getBoundingClientRect();
    const clientX = evt.touches ? evt.touches[0].clientX : evt.clientX;
    const clientY = evt.touches ? evt.touches[0].clientY : evt.clientY;
    return {
      x: Math.max(0, Math.min(1, (clientX - rect.left) / rect.width)),
      y: Math.max(0, Math.min(1, (clientY - rect.top) / rect.height))
    };
  },

  pushHistory() {
    this.undoStack.push(JSON.stringify(this.annotations));
    if (this.undoStack.length > 40) this.undoStack.shift();
    this.redoStack = [];
    this.hasUnsavedChanges = true;
    this.updateWindowTitle();
  },

  undo() {
    if (!this.undoStack.length) return;
    this.redoStack.push(JSON.stringify(this.annotations));
    this.annotations = JSON.parse(this.undoStack.pop());
    this.render();
    this.renderActivePage();
  },

  redo() {
    if (!this.redoStack.length) return;
    this.undoStack.push(JSON.stringify(this.annotations));
    this.annotations = JSON.parse(this.redoStack.pop());
    this.render();
    this.renderActivePage();
  },

  deleteAnnotation(id) {
    const list = this.annotations[this.currentPage] || [];
    this.pushHistory();
    this.annotations[this.currentPage] = list.filter(a => a.id !== id);
    if (this.selectedAnnotId === id) this.selectedAnnotId = null;
    this.render();
    this.renderActivePage();
  },

  clearPageAnnotations() {
    if (!confirm('Hapus semua anotasi pada halaman ini?')) return;
    this.pushHistory();
    this.annotations[this.currentPage] = [];
    this.selectedAnnotId = null;
    this.render();
    this.renderActivePage();
  },

  bindAnnotationEvents() {
    const canvas = document.getElementById('pdfcraft-annot-canvas');
    if (!canvas) return;

    const onDown = (e) => {
      e.preventDefault();
      const pt = this.getNormalizedPoint(e, canvas);

      if (this.activeTool === 'select') {
        const list = this.annotations[this.currentPage] || [];
        const hit = [...list].reverse().find(a => {
          const x = a.x || 0, y = a.y || 0, w = Math.max(a.w || 0.08, 0.06), h = Math.max(a.h || 0.05, 0.04);
          return pt.x >= x - 0.02 && pt.x <= x + w + 0.02 && pt.y >= y - 0.02 && pt.y <= y + h + 0.02;
        });
        this.selectedAnnotId = hit ? hit.id : null;
        this.redrawAnnotations();
        if (this.showInspector) this.render();
        return;
      }

      if (this.activeTool === 'text') {
        const text = prompt('Masukkan teks yang ingin ditambahkan pada PDF:');
        if (!text || !text.trim()) return;
        this.pushHistory();
        if (!this.annotations[this.currentPage]) this.annotations[this.currentPage] = [];
        this.annotations[this.currentPage].push({
          id: 'an_' + Date.now(),
          type: 'text',
          x: pt.x,
          y: pt.y,
          w: 0.25,
          h: 0.04,
          text: text.trim(),
          color: this.activeColor,
          size: this.activeSize
        });
        this.render();
        this.renderActivePage();
        return;
      }

      if (this.activeTool === 'note') {
        const text = prompt('Tulis catatan (Sticky Note) untuk dokumen ini:');
        if (!text || !text.trim()) return;
        this.pushHistory();
        if (!this.annotations[this.currentPage]) this.annotations[this.currentPage] = [];
        this.annotations[this.currentPage].push({
          id: 'an_' + Date.now(),
          type: 'note',
          x: pt.x,
          y: pt.y,
          w: 0.24,
          h: 0.07,
          text: text.trim(),
          color: '#f59e0b',
          size: 12
        });
        this.render();
        this.renderActivePage();
        return;
      }

      if (this.activeTool === 'stamp') {
        this.pushHistory();
        if (!this.annotations[this.currentPage]) this.annotations[this.currentPage] = [];
        const stampColors = { APPROVED: '#059669', CONFIDENTIAL: '#dc2626', DRAFT: '#d97706', REVIEWED: '#2563eb', REJECTED: '#b91c1c' };
        this.annotations[this.currentPage].push({
          id: 'an_' + Date.now(),
          type: 'stamp',
          x: Math.max(0.02, pt.x - 0.1),
          y: Math.max(0.02, pt.y - 0.03),
          w: 0.22,
          h: 0.06,
          text: this.activeStamp,
          color: stampColors[this.activeStamp] || '#dc2626',
          size: 18
        });
        this.render();
        this.renderActivePage();
        return;
      }

      // Drag tools: pen, highlight, rect
      this._isDrawing = true;
      this._dragStart = pt;
      if (this.activeTool === 'pen') {
        this._currentDraft = {
          id: 'an_' + Date.now(),
          type: 'pen',
          color: this.activeColor,
          size: Math.max(2, Math.round(this.activeSize / 5)),
          points: [pt]
        };
      } else {
        this._currentDraft = {
          id: 'an_' + Date.now(),
          type: this.activeTool,
          x: pt.x,
          y: pt.y,
          w: 0,
          h: 0,
          color: this.activeTool === 'highlight' ? '#facc15' : this.activeColor,
          size: Math.max(2, Math.round(this.activeSize / 6))
        };
      }
    };

    const onMove = (e) => {
      if (!this._isDrawing || !this._currentDraft) return;
      e.preventDefault();
      const pt = this.getNormalizedPoint(e, canvas);

      if (this._currentDraft.type === 'pen') {
        this._currentDraft.points.push(pt);
      } else {
        this._currentDraft.x = Math.min(this._dragStart.x, pt.x);
        this._currentDraft.y = Math.min(this._dragStart.y, pt.y);
        this._currentDraft.w = Math.abs(pt.x - this._dragStart.x);
        this._currentDraft.h = Math.abs(pt.y - this._dragStart.y);
      }
      this.redrawAnnotations();
    };

    const onUp = () => {
      if (!this._isDrawing || !this._currentDraft) return;
      this._isDrawing = false;
      const draft = this._currentDraft;
      this._currentDraft = null;

      const valid = draft.type === 'pen'
        ? draft.points.length > 1
        : (draft.w > 0.008 && draft.h > 0.005);

      if (valid) {
        if (draft.type === 'pen') {
          const xs = draft.points.map(p => p.x);
          const ys = draft.points.map(p => p.y);
          draft.x = Math.min(...xs);
          draft.y = Math.min(...ys);
          draft.w = Math.max(...xs) - draft.x;
          draft.h = Math.max(...ys) - draft.y;
        }
        this.pushHistory();
        if (!this.annotations[this.currentPage]) this.annotations[this.currentPage] = [];
        this.annotations[this.currentPage].push(draft);
        this.render();
        this.renderActivePage();
      } else {
        this.redrawAnnotations();
      }
    };

    canvas.addEventListener('mousedown', onDown);
    canvas.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
    canvas.addEventListener('touchstart', onDown, { passive: false });
    canvas.addEventListener('touchmove', onMove, { passive: false });
    window.addEventListener('touchend', onUp);
  },

  redrawAnnotations() {
    const canvas = document.getElementById('pdfcraft-annot-canvas');
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    const W = canvas.width;
    const H = canvas.height;
    ctx.clearRect(0, 0, W, H);

    const list = [...(this.annotations[this.currentPage] || [])];
    if (this._currentDraft) list.push(this._currentDraft);

    list.forEach((a) => {
      ctx.save();
      if (a.type === 'highlight') {
        ctx.fillStyle = 'rgba(250, 204, 21, 0.38)';
        ctx.fillRect(a.x * W, a.y * H, a.w * W, a.h * H);
      } else if (a.type === 'rect') {
        ctx.strokeStyle = a.color || '#ef4444';
        ctx.lineWidth = Math.max(2, (a.size || 2) * (W / 600));
        ctx.strokeRect(a.x * W, a.y * H, a.w * W, a.h * H);
      } else if (a.type === 'pen' && a.points && a.points.length > 1) {
        ctx.strokeStyle = a.color || '#111827';
        ctx.lineWidth = Math.max(2, (a.size || 3) * (W / 600));
        ctx.lineCap = 'round';
        ctx.lineJoin = 'round';
        ctx.beginPath();
        ctx.moveTo(a.points[0].x * W, a.points[0].y * H);
        for (let i = 1; i < a.points.length; i++) {
          ctx.lineTo(a.points[i].x * W, a.points[i].y * H);
        }
        ctx.stroke();
      } else if (a.type === 'text') {
        const fontPx = Math.max(12, (a.size || 16) * (W / 595));
        ctx.font = `700 ${fontPx}px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif`;
        ctx.fillStyle = a.color || '#111827';
        ctx.textBaseline = 'top';
        ctx.fillText(a.text || '', a.x * W, a.y * H);
      } else if (a.type === 'note') {
        const bx = a.x * W, by = a.y * H;
        const bw = Math.max(120, a.w * W), bh = Math.max(46, a.h * H);
        ctx.fillStyle = 'rgba(254, 243, 199, 0.96)';
        ctx.strokeStyle = '#d97706';
        ctx.lineWidth = 2;
        ctx.fillRect(bx, by, bw, bh);
        ctx.strokeRect(bx, by, bw, bh);
        ctx.fillStyle = '#92400e';
        ctx.font = `700 ${Math.max(11, 11 * (W / 595))}px sans-serif`;
        ctx.textBaseline = 'top';
        ctx.fillText('💬 ' + (a.text || '').slice(0, 32), bx + 8, by + 8);
      } else if (a.type === 'stamp') {
        const bx = a.x * W, by = a.y * H;
        const bw = a.w * W, bh = a.h * H;
        ctx.strokeStyle = a.color || '#dc2626';
        ctx.lineWidth = Math.max(3, 3 * (W / 595));
        ctx.strokeRect(bx, by, bw, bh);
        ctx.fillStyle = a.color || '#dc2626';
        ctx.font = `900 ${Math.max(14, 18 * (W / 595))}px sans-serif`;
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText(a.text || 'APPROVED', bx + bw / 2, by + bh / 2);
      }

      if (a.id === this.selectedAnnotId) {
        ctx.strokeStyle = '#007aff';
        ctx.lineWidth = 2;
        ctx.setLineDash([5, 4]);
        ctx.strokeRect((a.x || 0) * W - 4, (a.y || 0) * H - 4, Math.max(0.06, a.w || 0.08) * W + 8, Math.max(0.04, a.h || 0.04) * H + 8);
      }
      ctx.restore();
    });
  },

  /* ------------------------------------------------------------------------
     Page Organizer (pdf-lib): Add, Rotate, Reorder, Delete, Merge, Split
     ------------------------------------------------------------------------ */
  async addBlankPage() {
    try {
      const merged = await this.commitAnnotationsToPdfBytes();
      const doc = await PDFLib.PDFDocument.load(merged);
      doc.insertPage(this.currentPage, [595.28, 841.89]);
      const nextBytes = await doc.save();
      this.annotations = {};
      this.hasUnsavedChanges = true;
      await this.loadPdfBytes(nextBytes, this.currentPage + 1);
    } catch (e) {
      alert('Gagal menambah halaman: ' + e.message);
    }
  },

  async rotateCurrentPage() {
    try {
      const merged = await this.commitAnnotationsToPdfBytes();
      const doc = await PDFLib.PDFDocument.load(merged);
      const page = doc.getPage(this.currentPage - 1);
      const curAngle = page.getRotation().angle || 0;
      page.setRotation(PDFLib.degrees((curAngle + 90) % 360));
      const nextBytes = await doc.save();
      this.annotations = {};
      this.hasUnsavedChanges = true;
      await this.loadPdfBytes(nextBytes, this.currentPage);
    } catch (e) {
      alert('Gagal memutar halaman: ' + e.message);
    }
  },

  async moveCurrentPage(delta) {
    const targetIdx = (this.currentPage - 1) + delta;
    if (targetIdx < 0 || targetIdx >= this.totalPages) return;
    try {
      const merged = await this.commitAnnotationsToPdfBytes();
      const src = await PDFLib.PDFDocument.load(merged);
      const out = await PDFLib.PDFDocument.create();
      const order = Array.from({ length: src.getPageCount() }, (_, i) => i);
      const curIdx = this.currentPage - 1;
      [order[curIdx], order[targetIdx]] = [order[targetIdx], order[curIdx]];
      const copied = await out.copyPages(src, order);
      copied.forEach(p => out.addPage(p));
      const nextBytes = await out.save();
      this.annotations = {};
      this.hasUnsavedChanges = true;
      await this.loadPdfBytes(nextBytes, targetIdx + 1);
    } catch (e) {
      alert('Gagal memindahkan halaman: ' + e.message);
    }
  },

  async deleteCurrentPage() {
    if (this.totalPages <= 1) {
      alert('Dokumen PDF minimal harus memiliki 1 halaman.');
      return;
    }
    if (!confirm(`Hapus halaman ${this.currentPage}?`)) return;
    try {
      const merged = await this.commitAnnotationsToPdfBytes();
      const doc = await PDFLib.PDFDocument.load(merged);
      doc.removePage(this.currentPage - 1);
      const nextBytes = await doc.save();
      this.annotations = {};
      this.hasUnsavedChanges = true;
      await this.loadPdfBytes(nextBytes, Math.max(1, this.currentPage - 1));
    } catch (e) {
      alert('Gagal menghapus halaman: ' + e.message);
    }
  },

  async mergeLocalPdf(event) {
    const file = event.target.files[0];
    if (!file) return;
    try {
      const buf = await file.arrayBuffer();
      const baseBytes = await this.commitAnnotationsToPdfBytes();
      const baseDoc = await PDFLib.PDFDocument.load(baseBytes);
      const otherDoc = await PDFLib.PDFDocument.load(buf);
      const copied = await baseDoc.copyPages(otherDoc, otherDoc.getPageIndices());
      copied.forEach(p => baseDoc.addPage(p));
      const nextBytes = await baseDoc.save();
      this.annotations = {};
      this.hasUnsavedChanges = true;
      await this.loadPdfBytes(nextBytes, this.currentPage);
      if (window.DokuIsland) {
        DokuIsland.info(`Berhasil menggabungkan "${file.name}" (+${copied.length} halaman)`, 'PDFCraft');
      }
    } catch (e) {
      alert('Gagal menggabungkan PDF: ' + e.message);
    }
    event.target.value = '';
  },

  async extractCurrentPage() {
    try {
      const merged = await this.commitAnnotationsToPdfBytes();
      const src = await PDFLib.PDFDocument.load(merged);
      const out = await PDFLib.PDFDocument.create();
      const [page] = await out.copyPages(src, [this.currentPage - 1]);
      out.addPage(page);
      const bytes = await out.save();
      const baseName = (this.currentFile.name || 'document.pdf').replace(/\.pdf$/i, '');
      this.triggerBlobDownload(bytes, `${baseName}_hal_${this.currentPage}.pdf`);
    } catch (e) {
      alert('Gagal mengekstrak halaman: ' + e.message);
    }
  },

  /* ------------------------------------------------------------------------
     Full-Text Search via PDF.js
     ------------------------------------------------------------------------ */
  async searchText(query) {
    this.searchQuery = (query || '').trim();
    this.searchResults = [];
    if (!this.searchQuery || !this.pdfJsDoc) {
      this.render();
      this.renderActivePage();
      return;
    }

    const q = this.searchQuery.toLowerCase();
    for (let p = 1; p <= this.totalPages; p++) {
      try {
        const page = await this.pdfJsDoc.getPage(p);
        const textContent = await page.getTextContent();
        const fullText = textContent.items.map(i => i.str).join(' ');
        const idx = fullText.toLowerCase().indexOf(q);
        if (idx !== -1) {
          const start = Math.max(0, idx - 24);
          const snippet = fullText.slice(start, start + 72);
          this.searchResults.push({ page: p, snippet });
        }
      } catch {}
    }

    this.showInspector = true;
    this.render();
    this.renderActivePage();
    if (this.searchResults.length > 0) {
      this.goToPage(this.searchResults[0].page);
    } else if (window.DokuIsland) {
      DokuIsland.info(`Teks "${this.searchQuery}" tidak ditemukan`, 'PDFCraft');
    }
  },

  /* ------------------------------------------------------------------------
     Non-Destructive Commit to Standard PDF Bytes (pdf-lib) & Save to Finder
     ------------------------------------------------------------------------ */
  hexToPdfRgb(hex) {
    const clean = (hex || '#111827').replace('#', '');
    const r = parseInt(clean.slice(0, 2), 16) / 255 || 0;
    const g = parseInt(clean.slice(2, 4), 16) / 255 || 0;
    const b = parseInt(clean.slice(4, 6), 16) / 255 || 0;
    return PDFLib.rgb(r, g, b);
  },

  async commitAnnotationsToPdfBytes() {
    if (!this.pdfBytes) return null;
    const hasAny = Object.values(this.annotations).some(arr => arr && arr.length > 0);
    if (!hasAny) return this.pdfBytes;

    const doc = await PDFLib.PDFDocument.load(this.pdfBytes);
    const fontBold = await doc.embedFont(PDFLib.StandardFonts.HelveticaBold);
    const fontReg = await doc.embedFont(PDFLib.StandardFonts.Helvetica);
    const pages = doc.getPages();

    for (const [pageStr, list] of Object.entries(this.annotations)) {
      const pIdx = parseInt(pageStr, 10) - 1;
      if (!list || !list.length || pIdx < 0 || pIdx >= pages.length) continue;
      const page = pages[pIdx];
      const { width, height } = page.getSize();

      for (const a of list) {
        const col = this.hexToPdfRgb(a.color);
        if (a.type === 'highlight') {
          page.drawRectangle({
            x: a.x * width,
            y: height - (a.y + a.h) * height,
            width: a.w * width,
            height: a.h * height,
            color: PDFLib.rgb(0.98, 0.8, 0.08),
            opacity: 0.36
          });
        } else if (a.type === 'rect') {
          page.drawRectangle({
            x: a.x * width,
            y: height - (a.y + a.h) * height,
            width: a.w * width,
            height: a.h * height,
            borderColor: col,
            borderWidth: Math.max(1.5, a.size || 2)
          });
        } else if (a.type === 'pen' && a.points && a.points.length > 1) {
          for (let i = 1; i < a.points.length; i++) {
            const p0 = a.points[i - 1];
            const p1 = a.points[i];
            page.drawLine({
              start: { x: p0.x * width, y: height - p0.y * height },
              end: { x: p1.x * width, y: height - p1.y * height },
              thickness: Math.max(1.5, a.size || 2.5),
              color: col
            });
          }
        } else if (a.type === 'text') {
          const sz = Math.max(9, a.size || 14);
          page.drawText(a.text || '', {
            x: a.x * width,
            y: height - a.y * height - sz,
            size: sz,
            font: fontBold,
            color: col
          });
        } else if (a.type === 'note') {
          const bx = a.x * width;
          const bw = Math.max(120, (a.w || 0.24) * width);
          const bh = Math.max(36, (a.h || 0.06) * height);
          const by = height - a.y * height - bh;
          page.drawRectangle({
            x: bx, y: by, width: bw, height: bh,
            color: PDFLib.rgb(0.996, 0.95, 0.78),
            borderColor: PDFLib.rgb(0.85, 0.47, 0.02),
            borderWidth: 1.5
          });
          page.drawText('NOTE: ' + (a.text || '').slice(0, 40), {
            x: bx + 6, y: by + bh - 16,
            size: 9.5, font: fontReg, color: PDFLib.rgb(0.57, 0.25, 0.05)
          });
        } else if (a.type === 'stamp') {
          const bx = a.x * width;
          const bw = (a.w || 0.22) * width;
          const bh = (a.h || 0.06) * height;
          const by = height - a.y * height - bh;
          page.drawRectangle({
            x: bx, y: by, width: bw, height: bh,
            borderColor: col, borderWidth: 2.5
          });
          page.drawText(a.text || 'APPROVED', {
            x: bx + 10, y: by + bh / 2 - 6,
            size: 14, font: fontBold, color: col
          });
        }
      }
    }

    return await doc.save();
  },

  uint8ToBase64(bytes) {
    let binary = '';
    const chunk = 0x8000;
    for (let i = 0; i < bytes.length; i += chunk) {
      binary += String.fromCharCode.apply(null, bytes.subarray(i, i + chunk));
    }
    return btoa(binary);
  },

  async saveToFinder() {
    try {
      const finalBytes = await this.commitAnnotationsToPdfBytes();
      if (!finalBytes) return;
      const dataUrl = 'data:application/pdf;base64,' + this.uint8ToBase64(finalBytes);
      const headers = { ...this.getAuthHeaders(), 'Content-Type': 'application/json' };

      if (this.currentFile && this.currentFile.id) {
        const res = await fetch(`/api/files/${this.currentFile.id}/content`, {
          method: 'PUT',
          headers,
          body: JSON.stringify({ content: dataUrl })
        });
        const data = await res.json();
        if (!data.success) throw new Error(data.error || 'Gagal menyimpan');
      } else {
        const name = prompt('Nama berkas PDF untuk disimpan di Finder:', this.currentFile.name || 'Dokumen.pdf');
        if (!name || !name.trim()) return;
        const finalName = name.trim().toLowerCase().endsWith('.pdf') ? name.trim() : name.trim() + '.pdf';
        const res = await fetch('/api/files', {
          method: 'POST',
          headers,
          body: JSON.stringify({
            name: finalName,
            extension: 'pdf',
            parent_id: 'root',
            content: dataUrl
          })
        });
        const data = await res.json();
        if (!data.success) throw new Error(data.error || 'Gagal membuat berkas');
        this.currentFile = { id: data.file ? data.file.id : null, name: finalName };
      }

      this.annotations = {};
      this.undoStack = [];
      this.redoStack = [];
      this.hasUnsavedChanges = false;
      await this.loadPdfBytes(finalBytes, this.currentPage);
      if (window.DokuFileManager && DokuFileManager.refresh) DokuFileManager.refresh();
      if (window.DokuIsland) {
        DokuIsland.info(`"${this.currentFile.name}" berhasil disimpan ke Finder!`, 'PDFCraft');
      }
    } catch (e) {
      alert('Gagal menyimpan PDF ke Finder: ' + e.message);
    }
  },

  triggerBlobDownload(bytes, filename) {
    const blob = new Blob([bytes], { type: 'application/pdf' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  },

  async downloadEditedPdf() {
    try {
      const finalBytes = await this.commitAnnotationsToPdfBytes();
      if (!finalBytes) return;
      this.triggerBlobDownload(finalBytes, (this.currentFile && this.currentFile.name) || 'dokumen.pdf');
    } catch (e) {
      alert('Gagal mengunduh PDF: ' + e.message);
    }
  },

  /* ------------------------------------------------------------------------
     Finder Modal File Picker
     ------------------------------------------------------------------------ */
  async openFromFileManagerModal() {
    let existing = document.getElementById('pdfcraft-finder-modal');
    if (existing) existing.remove();

    try {
      const res = await fetch('/api/files', { headers: this.getAuthHeaders() });
      const data = await res.json();
      const allFiles = data.files || [];
      this._availablePdfs = allFiles.filter(f => !f.is_folder && (f.extension === 'pdf' || (f.name && f.name.toLowerCase().endsWith('.pdf'))));
    } catch {
      this._availablePdfs = [];
    }

    const overlay = document.createElement('div');
    overlay.id = 'pdfcraft-finder-modal';
    overlay.style.cssText = 'position:fixed; inset:0; background:rgba(0,0,0,0.5); backdrop-filter:blur(8px); z-index:999999; display:flex; align-items:center; justify-content:center;';
    overlay.innerHTML = `
      <div style="width:460px; max-width:calc(100vw - 24px); background:var(--window-bg); border:1px solid var(--window-border); border-radius:16px; padding:20px; box-shadow:0 30px 70px rgba(0,0,0,0.4);">
        <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:14px;">
          <h3 style="font-size:15px; font-weight:700;">📂 Pilih Dokumen PDF dari Finder</h3>
          <button class="traffic-btn btn-close" onclick="document.getElementById('pdfcraft-finder-modal').remove()"></button>
        </div>
        <div style="max-height:300px; overflow-y:auto; display:flex; flex-direction:column; gap:6px; margin-bottom:14px;">
          ${this._availablePdfs.length === 0 ? `
            <div style="text-align:center; padding:24px; color:var(--text-muted); font-size:12px;">
              Belum ada berkas <code>.pdf</code> di Finder Anda. Anda dapat membuat PDF baru atau mengunggah berkas lokal.
            </div>
          ` : this._availablePdfs.map((f, idx) => `
            <div onclick="DokuPdfViewer.selectFinderPdf(${idx})"
              style="display:flex; align-items:center; justify-content:space-between; padding:10px 12px; border-radius:10px; background:var(--card-bg); border:1px solid var(--window-border); cursor:pointer;">
              <div style="display:flex; align-items:center; gap:10px;">
                <span style="font-size:20px;">📄</span>
                <div>
                  <div style="font-size:13px; font-weight:600;">${f.name}</div>
                  <div style="font-size:10.5px; color:var(--text-muted);">${Math.max(1, Math.round((f.size_bytes || 0) / 1024))} KB</div>
                </div>
              </div>
              <span class="mac-btn primary" style="font-size:11px; padding:2px 9px;">Buka</span>
            </div>
          `).join('')}
        </div>
        <div style="display:flex; justify-content:flex-end;">
          <button class="mac-btn" onclick="document.getElementById('pdfcraft-finder-modal').remove()">Tutup</button>
        </div>
      </div>
    `;
    document.body.appendChild(overlay);
  },

  selectFinderPdf(idx) {
    const f = this._availablePdfs[idx];
    const modal = document.getElementById('pdfcraft-finder-modal');
    if (modal) modal.remove();
    if (f) this.loadFile(f);
  }
};
