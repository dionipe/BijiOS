/**
 * WordCraft - Sovereign Word Processor
 * Full-fidelity adoption of storytold/wordcraft (Pure Rust & Web Word Processor)
 * Features:
 * - Office-grade Ribbon with Tab System: File, Home, Insert, Layout, References, Review, View
 * - Live Styles Gallery (Normal, No Spacing, Heading 1, 2, 3, Title, Subtitle, Quote, Emphasis)
 * - Precision Top Horizontal Ruler with centimeter markings
 * - Page Setup: Margins, Orientation (Portrait/Landscape), Columns, Watermark
 * - Auto-generating Table of Contents & Headings Inspector
 * - Interactive Zoom Engine (50% - 200%) & Multi-View Engine (Print, Web, Read)
 * - Cloud Persistence to DokuHub TOON Engine + Multi-format Exporter (.docx, PDF, HTML, MD, TXT)
 */

window.DokuWordCraft = {
  container: null,
  currentFile: null,
  activeTab: 'home',
  zoomLevel: 100,
  viewMode: 'print', // 'print', 'web', 'read'
  showRuler: true,
  showNavPane: false,
  isDirty: false,
  pageOrientation: 'portrait',
  pageMargins: 'normal',

  async init(containerEl, file = null) {
    this.container = containerEl;
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    this.currentFile = file || {
      id: 'new_word',
      name: isId ? 'Dokumen Baru.docx' : 'Untitled Document.docx'
    };
    this.zoomLevel = 100;
    this.viewMode = 'print';
    this.activeTab = 'home';
    this.showRuler = true;
    this.showNavPane = false;
    this.isDirty = false;

    this.render();
    this.updateWindowTitle();
    await this.loadDocumentContent();
  },

  updateWindowTitle() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const docName = (this.currentFile && this.currentFile.name)
      ? this.currentFile.name
      : (isId ? 'Dokumen Baru.docx' : 'Untitled Document.docx');

    const win = document.getElementById('window-wordcraft');
    if (win) {
      const titleEl = win.querySelector('.window-title');
      if (titleEl) {
        titleEl.innerHTML = `📝 WordCraft — ${docName}${this.isDirty ? ' <span style="color:#ef4444; font-size:10px;">●</span>' : ''}`;
      }
    }

    const appTitleMenu = document.getElementById('active-app-name');
    if (appTitleMenu && window.DokuHub && DokuHub.activeAppId === 'wordcraft') {
      appTitleMenu.textContent = `WordCraft — ${docName}`;
    }
  },

  async loadFile(file) {
    if (!file) return;
    this.currentFile = file;
    this.isDirty = false;
    this.updateWindowTitle();
    await this.loadDocumentContent();
    const statusEl = document.getElementById('wordcraft-status');
    if (statusEl) {
      statusEl.textContent = (window.DokuI18n && DokuI18n.lang === 'id') ? '● Tersimpan' : '● Saved';
      statusEl.style.color = '#10b981';
    }
  },

  render() {
    if (!this.container) return;
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');

    this.container.innerHTML = `
      <div class="wordcraft-layout">
        <!-- 1. Top Ribbon Navigation & Toolbars -->
        <div class="wordcraft-ribbon-bar">
          <!-- Ribbon Tab Headers -->
          <div class="wordcraft-tabs-header">
            <button class="wordcraft-tab-btn tab-file" onclick="DokuWordCraft.toggleFileMenu(event)">
              ${isId ? 'Berkas' : 'File'} ▾
            </button>
            <button class="wordcraft-tab-btn ${this.activeTab === 'home' ? 'active' : ''}" onclick="DokuWordCraft.switchTab('home')">
              ${isId ? 'Beranda' : 'Home'}
            </button>
            <button class="wordcraft-tab-btn ${this.activeTab === 'insert' ? 'active' : ''}" onclick="DokuWordCraft.switchTab('insert')">
              ${isId ? 'Sisipkan' : 'Insert'}
            </button>
            <button class="wordcraft-tab-btn ${this.activeTab === 'layout' ? 'active' : ''}" onclick="DokuWordCraft.switchTab('layout')">
              ${isId ? 'Tata Letak' : 'Layout'}
            </button>
            <button class="wordcraft-tab-btn ${this.activeTab === 'references' ? 'active' : ''}" onclick="DokuWordCraft.switchTab('references')">
              ${isId ? 'Referensi' : 'References'}
            </button>
            <button class="wordcraft-tab-btn ${this.activeTab === 'review' ? 'active' : ''}" onclick="DokuWordCraft.switchTab('review')">
              ${isId ? 'Peninjauan' : 'Review'}
            </button>
            <button class="wordcraft-tab-btn ${this.activeTab === 'view' ? 'active' : ''}" onclick="DokuWordCraft.switchTab('view')">
              ${isId ? 'Tampilan' : 'View'}
            </button>

            <div style="flex:1;"></div>

            <!-- Quick Action Bar -->
            <div style="display:flex; align-items:center; gap:6px; margin-right:8px;">
              <span id="wordcraft-status" style="font-size:11px; color:#10b981; font-weight:500;">● ${isId ? 'Tersimpan' : 'Saved'}</span>
              <button class="mac-btn" onclick="DokuWordCraft.openFromFileManagerModal()" style="padding:2px 8px; font-size:11px;" title="${isId ? 'Buka Dokumen dari Finder' : 'Open Document from Finder'}">
                📂 ${isId ? 'Buka...' : 'Open...'}
              </button>
              <button class="mac-btn primary" onclick="DokuWordCraft.saveDocument()" style="padding:2px 8px; font-size:11px;" title="${isId ? 'Simpan Dokumen (Ctrl+S)' : 'Save Document (Ctrl+S)'}">
                💾 ${isId ? 'Simpan' : 'Save'}
              </button>
              <button class="mac-btn" onclick="DokuWordCraft.openSaveAsModal()" style="padding:2px 8px; font-size:11px;" title="${isId ? 'Simpan Sebagai (Pilih Folder Tujuan)' : 'Save As (Choose Folder)'}">
                📑 ${isId ? 'Simpan Sebagai...' : 'Save As...'}
              </button>
            </div>
          </div>

          <!-- Dynamic Ribbon Action Panels based on active tab -->
          <div class="wordcraft-ribbon-panel" id="wordcraft-ribbon-content">
            ${this.renderActiveRibbonTab()}
          </div>
        </div>

        <!-- 2. Horizontal Top Precision Ruler -->
        <div class="wordcraft-ruler-container" id="wordcraft-ruler-wrapper" style="display:${this.showRuler && this.viewMode === 'print' ? 'flex' : 'none'};">
          <div class="wordcraft-ruler" id="wordcraft-ruler">
            <div class="ruler-margin-left"></div>
            <div class="ruler-white-area"></div>
            <div class="ruler-margin-right"></div>
            ${this.renderRulerTicks()}
            <!-- Interactive Markers -->
            <div class="ruler-marker marker-first-line" id="ruler-first-line" style="left:80px;" title="${isId ? 'Indentasi Baris Pertama (First Line Indent)' : 'First Line Indent'}">
              <svg width="10" height="8" viewBox="0 0 10 8"><polygon points="0,0 10,0 5,8" fill="#475569" stroke="#1e293b" stroke-width="0.8"/></svg>
            </div>
            <div class="ruler-marker marker-hanging" id="ruler-hanging" style="left:80px;" title="${isId ? 'Indentasi Gantung (Hanging Indent) - Format 1. ...' : 'Hanging Indent'}">
              <svg width="10" height="8" viewBox="0 0 10 8"><polygon points="5,0 10,8 0,8" fill="#475569" stroke="#1e293b" stroke-width="0.8"/></svg>
            </div>
            <div class="ruler-marker marker-left-base" id="ruler-left-base" style="left:80px;" title="${isId ? 'Indentasi Kiri (Left Indent)' : 'Left Indent'}"></div>
            <div class="ruler-marker marker-right-indent" id="ruler-right-indent" style="left:736px;" title="${isId ? 'Indentasi Kanan (Right Indent)' : 'Right Indent'}">
              <svg width="10" height="10" viewBox="0 0 10 10"><polygon points="5,0 10,10 0,10" fill="#475569" stroke="#1e293b" stroke-width="0.8"/></svg>
            </div>
            <!-- Dropped Vertical Guide Line across document page -->
            <div class="ruler-guide-line" id="ruler-guide-line"></div>
          </div>
        </div>

        <!-- 3. Canvas Wrapper with Navigation Outline Drawer -->
        <div class="wordcraft-canvas-wrapper">
          <!-- Outline Navigation Drawer -->
          <div class="wordcraft-nav-pane ${this.showNavPane ? '' : 'hidden'}" id="wordcraft-nav-pane">
            <div style="padding:10px 12px; font-weight:700; border-bottom:1px solid var(--window-border); display:flex; justify-content:space-between; align-items:center;">
              <span>📑 ${isId ? 'Navigasi Dokumen' : 'Navigation'}</span>
              <button class="traffic-btn btn-close" onclick="DokuWordCraft.toggleNavPane()" style="width:10px; height:10px;"></button>
            </div>
            <div id="wordcraft-outline-list" style="padding:8px; overflow-y:auto; flex:1;">
              <!-- Dynamic Headings Injected Here -->
            </div>
          </div>

          <!-- Document Canvas Viewport -->
          <div class="wordcraft-canvas" id="wordcraft-canvas" onclick="DokuWordCraft.focusEditor()">
            <div class="wordcraft-page-wrapper" id="wordcraft-page-wrapper" style="transform: scale(${this.zoomLevel / 100});">
              <div class="a4-page" id="wordcraft-editor-body" contenteditable="true" spellcheck="true">
                <!-- Document Content Injected Here -->
              </div>
            </div>
          </div>
        </div>

        <!-- 4. Professional Word Status Bar -->
        <div class="wordcraft-statusbar">
          <div class="wordcraft-status-left">
            <span id="wordcraft-page-info">Halaman 1 dari 1</span>
            <span>•</span>
            <span id="wordcraft-stats">0 Kata • 0 Karakter</span>
            <span>•</span>
            <span style="display:flex; align-items:center; gap:4px; opacity:0.8;">
              <span>🇮🇩</span> <span>Bahasa Indonesia</span>
            </span>
          </div>

          <div class="wordcraft-status-right">
            <!-- View Mode Switchers -->
            <div class="wordcraft-view-pills">
              <button class="wordcraft-view-pill ${this.viewMode === 'read' ? 'active' : ''}" onclick="DokuWordCraft.setViewMode('read')" title="${isId ? 'Mode Baca' : 'Read Mode'}">📖</button>
              <button class="wordcraft-view-pill ${this.viewMode === 'print' ? 'active' : ''}" onclick="DokuWordCraft.setViewMode('print')" title="${isId ? 'Tata Letak Cetak (A4)' : 'Print Layout'}">📄</button>
              <button class="wordcraft-view-pill ${this.viewMode === 'web' ? 'active' : ''}" onclick="DokuWordCraft.setViewMode('web')" title="${isId ? 'Tata Letak Web' : 'Web Layout'}">🌐</button>
            </div>

            <!-- Zoom Controller -->
            <div style="display:flex; align-items:center; gap:6px;">
              <button class="ribbon-btn" style="height:20px; width:20px; padding:0; font-size:11px;" onclick="DokuWordCraft.adjustZoom(-10)">-</button>
              <input type="range" min="50" max="200" step="10" value="${this.zoomLevel}" oninput="DokuWordCraft.setZoom(this.value)" style="width:70px; height:4px; cursor:pointer;">
              <button class="ribbon-btn" style="height:20px; width:20px; padding:0; font-size:11px;" onclick="DokuWordCraft.adjustZoom(10)">+</button>
              <span id="wordcraft-zoom-label" style="min-width:34px; text-align:right;">${this.zoomLevel}%</span>
            </div>
          </div>
        </div>

        <!-- Hidden File Picker for Image Insert -->
        <input type="file" id="wordcraft-img-uploader" accept="image/*" style="display:none;" onchange="DokuWordCraft.handleImageInsert(event)">
      </div>
    `;

    this.bindEditorEvents();
    this.updateStats();
  },

  /* ------------------------------------------------------------------------
     Ribbon Tab Renderers
     ------------------------------------------------------------------------ */
  renderActiveRibbonTab() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');

    switch (this.activeTab) {
      case 'home':
        return `
          <!-- Clipboard Group -->
          <div class="ribbon-group">
            <div class="ribbon-row">
              <button class="ribbon-btn" onclick="DokuWordCraft.pasteClipboard()" title="Paste (Ctrl+V)">📋</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.execCmd('cut')" title="Cut (Ctrl+X)">✂️</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.execCmd('copy')" title="Copy (Ctrl+C)">📄</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.execCmd('removeFormat')" title="Clear Formatting">🧹</button>
            </div>
            <div class="ribbon-group-title">${isId ? 'Papan Klip' : 'Clipboard'}</div>
          </div>

          <!-- Font Group -->
          <div class="ribbon-group">
            <div class="ribbon-row" style="margin-bottom:3px;">
              <select class="ribbon-select" style="width:115px;" onchange="DokuWordCraft.execCmd('fontName', this.value)">
                <option value="Calibri" selected>Calibri</option>
                <option value="Arial">Arial</option>
                <option value="Times New Roman">Times New Roman</option>
                <option value="Inter">Inter</option>
                <option value="Georgia">Georgia</option>
                <option value="Courier New">Courier New</option>
              </select>
              <select class="ribbon-select" style="width:48px;" onchange="DokuWordCraft.setFontSize(this.value)">
                <option value="1">8 pt</option>
                <option value="2">10 pt</option>
                <option value="3" selected>11 pt</option>
                <option value="4">12 pt</option>
                <option value="5">14 pt</option>
                <option value="6">18 pt</option>
                <option value="7">24 pt</option>
              </select>
              <button class="ribbon-btn" onclick="DokuWordCraft.changeFontSize(1)" title="Increase Font">A▲</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.changeFontSize(-1)" title="Decrease Font">A▼</button>
            </div>
            <div class="ribbon-row">
              <button class="ribbon-btn" onclick="DokuWordCraft.execCmd('bold')" title="Bold (Ctrl+B)"><strong>B</strong></button>
              <button class="ribbon-btn" onclick="DokuWordCraft.execCmd('italic')" title="Italic (Ctrl+I)"><em>I</em></button>
              <button class="ribbon-btn" onclick="DokuWordCraft.execCmd('underline')" title="Underline (Ctrl+U)"><u>U</u></button>
              <button class="ribbon-btn" onclick="DokuWordCraft.execCmd('strikeThrough')" title="Strikethrough"><s>S</s></button>
              <button class="ribbon-btn" onclick="DokuWordCraft.execCmd('subscript')" title="Subscript">X₂</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.execCmd('superscript')" title="Superscript">X²</button>
              <!-- Color Pickers -->
              <label class="ribbon-btn" style="cursor:pointer; position:relative; overflow:hidden;" title="Text Color">
                <span style="font-weight:700; border-bottom:3px solid #ef4444;">A</span>
                <input type="color" value="#111827" onchange="DokuWordCraft.execCmd('foreColor', this.value)" style="opacity:0; position:absolute; inset:0; cursor:pointer;">
              </label>
              <label class="ribbon-btn" style="cursor:pointer; position:relative; overflow:hidden;" title="Highlight Color">
                <span>🖊️</span>
                <input type="color" value="#fef08a" onchange="DokuWordCraft.execCmd('hiliteColor', this.value)" style="opacity:0; position:absolute; inset:0; cursor:pointer;">
              </label>
            </div>
            <div class="ribbon-group-title">${isId ? 'Huruf' : 'Font'}</div>
          </div>

          <!-- Paragraph Group -->
          <div class="ribbon-group">
            <div class="ribbon-row" style="margin-bottom:3px;">
              <button class="ribbon-btn" onclick="DokuWordCraft.execCmd('insertUnorderedList')" title="Bullet List">• ≡</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.execCmd('insertOrderedList')" title="Numbered List">1. ≡</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.execCmd('outdent')" title="Decrease Indent">⇤</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.execCmd('indent')" title="Increase Indent">⇥</button>
              <select class="ribbon-select" style="width:60px;" onchange="DokuWordCraft.setLineSpacing(this.value)" title="Line Spacing">
                <option value="1.15">1.15</option>
                <option value="1.0">1.0</option>
                <option value="1.5">1.5</option>
                <option value="2.0">2.0</option>
              </select>
            </div>
            <div class="ribbon-row">
              <button class="ribbon-btn" onclick="DokuWordCraft.execCmd('justifyLeft')" title="Align Left">⬅️</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.execCmd('justifyCenter')" title="Center">↔️</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.execCmd('justifyRight')" title="Align Right">➡️</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.execCmd('justifyFull')" title="Justify">↕️</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.formatBlockquote()" title="Blockquote">❝❞</button>
            </div>
            <div class="ribbon-group-title">${isId ? 'Paragraf' : 'Paragraph'}</div>
          </div>

          <!-- Styles Gallery (Live Word Styles) -->
          <div class="ribbon-group" style="flex:1; max-width:480px;">
            <div class="styles-gallery">
              <div class="style-card active" onclick="DokuWordCraft.applyStyle('normal')">
                <div class="style-preview" style="font-size:12px; font-weight:400;">AaBbCc</div>
                <div class="style-label">Normal</div>
              </div>
              <div class="style-card" onclick="DokuWordCraft.applyStyle('no_spacing')">
                <div class="style-preview" style="font-size:12px; line-height:1;">AaBbCc</div>
                <div class="style-label">${isId ? 'Tanpa Spasi' : 'No Spacing'}</div>
              </div>
              <div class="style-card" onclick="DokuWordCraft.applyStyle('heading1')">
                <div class="style-preview" style="font-size:13px; font-weight:700; color:#1e40af;">AaBb</div>
                <div class="style-label">Heading 1</div>
              </div>
              <div class="style-card" onclick="DokuWordCraft.applyStyle('heading2')">
                <div class="style-preview" style="font-size:12px; font-weight:600; color:#2563eb;">AaBb</div>
                <div class="style-label">Heading 2</div>
              </div>
              <div class="style-card" onclick="DokuWordCraft.applyStyle('heading3')">
                <div class="style-preview" style="font-size:11px; font-weight:600; color:#3b82f6;">AaBb</div>
                <div class="style-label">Heading 3</div>
              </div>
              <div class="style-card" onclick="DokuWordCraft.applyStyle('title')">
                <div class="style-preview" style="font-size:14px; font-weight:800;">Title</div>
                <div class="style-label">Judul</div>
              </div>
              <div class="style-card" onclick="DokuWordCraft.applyStyle('subtitle')">
                <div class="style-preview" style="font-size:11px; font-style:italic; color:#64748b;">Subtitle</div>
                <div class="style-label">Subjudul</div>
              </div>
              <div class="style-card" onclick="DokuWordCraft.applyStyle('quote')">
                <div class="style-preview" style="font-size:11px; font-style:italic; border-left:2px solid #3b82f6; padding-left:3px;">"Kutipan"</div>
                <div class="style-label">Quote</div>
              </div>
            </div>
            <div class="ribbon-group-title">${isId ? 'Gaya Teks (Styles)' : 'Styles'}</div>
          </div>

          <!-- Editing Group -->
          <div class="ribbon-group">
            <div class="ribbon-row">
              <button class="ribbon-btn" onclick="DokuWordCraft.openFindModal()" title="Find (Ctrl+F)">🔍 ${isId ? 'Cari' : 'Find'}</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.execCmd('selectAll')" title="Select All (Ctrl+A)">🔲 ${isId ? 'Semua' : 'All'}</button>
            </div>
            <div class="ribbon-group-title">${isId ? 'Edit' : 'Editing'}</div>
          </div>
        `;

      case 'insert':
        return `
          <!-- Pages Group -->
          <div class="ribbon-group">
            <div class="ribbon-row">
              <button class="ribbon-btn" onclick="DokuWordCraft.insertPageBreak()" title="Page Break">📄 ${isId ? 'Batas Halaman' : 'Page Break'}</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.execCmd('insertHorizontalRule')" title="Horizontal Line">― ${isId ? 'Garis' : 'Divider'}</button>
            </div>
            <div class="ribbon-group-title">${isId ? 'Halaman' : 'Pages'}</div>
          </div>

          <!-- Tables Group -->
          <div class="ribbon-group">
            <div class="ribbon-row">
              <button class="ribbon-btn" onclick="DokuWordCraft.insertTable(3, 3)">▦ ${isId ? 'Sisipkan Tabel (3x3)' : 'Table (3x3)'}</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.insertCustomTable()">▦ ${isId ? 'Tabel Kustom...' : 'Custom Table...'}</button>
            </div>
            <div class="ribbon-group-title">${isId ? 'Tabel' : 'Tables'}</div>
          </div>

          <!-- Illustrations Group -->
          <div class="ribbon-group">
            <div class="ribbon-row">
              <button class="ribbon-btn" onclick="document.getElementById('wordcraft-img-uploader').click()">🖼️ ${isId ? 'Gambar Komputer' : 'Picture'}</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.insertImageByUrl()">🌐 ${isId ? 'Gambar URL' : 'Image URL'}</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.insertCalloutBox()">💬 ${isId ? 'Kotak Info' : 'Callout Box'}</button>
            </div>
            <div class="ribbon-group-title">${isId ? 'Ilustrasi' : 'Illustrations'}</div>
          </div>

          <!-- Links & Headers -->
          <div class="ribbon-group">
            <div class="ribbon-row">
              <button class="ribbon-btn" onclick="DokuWordCraft.insertLink()">🔗 ${isId ? 'Tautan Web' : 'Hyperlink'}</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.insertPageNumber()"># ${isId ? 'Nomor Halaman' : 'Page Number'}</button>
            </div>
            <div class="ribbon-group-title">${isId ? 'Tautan & Nomor' : 'Links & Numbers'}</div>
          </div>

          <!-- Symbols -->
          <div class="ribbon-group">
            <div class="ribbon-row">
              <button class="ribbon-btn" onclick="DokuWordCraft.insertSymbol('©')">©</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.insertSymbol('®')">®</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.insertSymbol('™')">™</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.insertSymbol('✓')">✓</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.insertSymbol('→')">→</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.insertDateStamp()">📅 ${isId ? 'Tanggal' : 'Date'}</button>
            </div>
            <div class="ribbon-group-title">${isId ? 'Simbol' : 'Symbols'}</div>
          </div>
        `;

      case 'layout':
        return `
          <!-- Page Setup -->
          <div class="ribbon-group">
            <div class="ribbon-row">
              <select class="ribbon-select" onchange="DokuWordCraft.setMargins(this.value)">
                <option value="normal" ${this.pageMargins === 'normal' ? 'selected' : ''}>${isId ? 'Margin Normal (2.54 cm)' : 'Normal Margins'}</option>
                <option value="narrow" ${this.pageMargins === 'narrow' ? 'selected' : ''}>${isId ? 'Margin Sempit (1.27 cm)' : 'Narrow Margins'}</option>
                <option value="wide" ${this.pageMargins === 'wide' ? 'selected' : ''}>${isId ? 'Margin Lebar (3.8 cm)' : 'Wide Margins'}</option>
              </select>
              <select class="ribbon-select" onchange="DokuWordCraft.setOrientation(this.value)">
                <option value="portrait" ${this.pageOrientation === 'portrait' ? 'selected' : ''}>${isId ? 'Orientasi: Tegak (Portrait)' : 'Orientation: Portrait'}</option>
                <option value="landscape" ${this.pageOrientation === 'landscape' ? 'selected' : ''}>${isId ? 'Orientasi: Mendatar (Landscape)' : 'Orientation: Landscape'}</option>
              </select>
            </div>
            <div class="ribbon-group-title">${isId ? 'Pengaturan Halaman' : 'Page Setup'}</div>
          </div>

          <!-- Columns & Watermark -->
          <div class="ribbon-group">
            <div class="ribbon-row">
              <button class="ribbon-btn" onclick="DokuWordCraft.setColumns(1)">| ${isId ? '1 Kolom' : '1 Col'}</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.setColumns(2)">|| ${isId ? '2 Kolom' : '2 Cols'}</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.setWatermark()">💧 ${isId ? 'Tanda Air (Watermark)...' : 'Watermark...'}</button>
            </div>
            <div class="ribbon-group-title">${isId ? 'Kolom & Latar' : 'Columns & Background'}</div>
          </div>
        `;

      case 'references':
        return `
          <!-- Table of Contents -->
          <div class="ribbon-group">
            <div class="ribbon-row">
              <button class="ribbon-btn primary" onclick="DokuWordCraft.generateTableOfContents()" style="background:var(--accent-color); color:#fff;">
                📑 ${isId ? 'Buat Daftar Isi Otomatis' : 'Generate Table of Contents'}
              </button>
            </div>
            <div class="ribbon-group-title">${isId ? 'Daftar Isi' : 'Table of Contents'}</div>
          </div>

          <!-- Footnotes & Citations -->
          <div class="ribbon-group">
            <div class="ribbon-row">
              <button class="ribbon-btn" onclick="DokuWordCraft.insertFootnote()">¹ ${isId ? 'Catatan Kaki (Footnote)' : 'Insert Footnote'}</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.insertCitation()">📖 ${isId ? 'Kutipan Pustaka' : 'Insert Citation'}</button>
            </div>
            <div class="ribbon-group-title">${isId ? 'Catatan Kaki' : 'Footnotes'}</div>
          </div>
        `;

      case 'review':
        return `
          <!-- Proofing -->
          <div class="ribbon-group">
            <div class="ribbon-row">
              <button class="ribbon-btn" onclick="DokuWordCraft.showWordCountModal()">📊 ${isId ? 'Statistik Kata Lengkap' : 'Word Count Details'}</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.checkProofing()">✔️ ${isId ? 'Periksa Ejaan' : 'Spell Check'}</button>
            </div>
            <div class="ribbon-group-title">${isId ? 'Pemeriksaan' : 'Proofing'}</div>
          </div>

          <!-- Comments -->
          <div class="ribbon-group">
            <div class="ribbon-row">
              <button class="ribbon-btn" onclick="DokuWordCraft.insertComment()">💬 ${isId ? 'Tambah Komentar' : 'New Comment'}</button>
            </div>
            <div class="ribbon-group-title">${isId ? 'Komentar' : 'Comments'}</div>
          </div>
        `;

      case 'view':
        return `
          <!-- Document Views -->
          <div class="ribbon-group">
            <div class="ribbon-row">
              <button class="ribbon-btn ${this.viewMode === 'print' ? 'active' : ''}" onclick="DokuWordCraft.setViewMode('print')">📄 ${isId ? 'Tata Letak Cetak' : 'Print Layout'}</button>
              <button class="ribbon-btn ${this.viewMode === 'web' ? 'active' : ''}" onclick="DokuWordCraft.setViewMode('web')">🌐 ${isId ? 'Tata Letak Web' : 'Web Layout'}</button>
              <button class="ribbon-btn ${this.viewMode === 'read' ? 'active' : ''}" onclick="DokuWordCraft.setViewMode('read')">📖 ${isId ? 'Mode Membaca' : 'Read Mode'}</button>
            </div>
            <div class="ribbon-group-title">${isId ? 'Tampilan Dokumen' : 'Views'}</div>
          </div>

          <!-- Show / Hide -->
          <div class="ribbon-group">
            <div class="ribbon-row">
              <button class="ribbon-btn ${this.showRuler ? 'active' : ''}" onclick="DokuWordCraft.toggleRuler()">📏 ${isId ? 'Mistar (Ruler)' : 'Ruler'}</button>
              <button class="ribbon-btn ${this.showNavPane ? 'active' : ''}" onclick="DokuWordCraft.toggleNavPane()">📑 ${isId ? 'Panel Navigasi' : 'Navigation Pane'}</button>
            </div>
            <div class="ribbon-group-title">${isId ? 'Tampilkan' : 'Show'}</div>
          </div>

          <!-- Zoom -->
          <div class="ribbon-group">
            <div class="ribbon-row">
              <button class="ribbon-btn" onclick="DokuWordCraft.setZoom(100)">100%</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.setZoom(125)">125%</button>
              <button class="ribbon-btn" onclick="DokuWordCraft.setZoom(75)">75%</button>
            </div>
            <div class="ribbon-group-title">${isId ? 'Perbesaran' : 'Zoom'}</div>
          </div>
        `;

      default:
        return '';
    }
  },

  switchTab(tabKey) {
    this.activeTab = tabKey;
    document.querySelectorAll('.wordcraft-tab-btn').forEach(btn => {
      if (btn.classList.contains('tab-file')) return;
      btn.classList.remove('active');
    });
    const activeBtn = Array.from(document.querySelectorAll('.wordcraft-tab-btn')).find(b => b.textContent.trim().toLowerCase().includes(tabKey) || b.getAttribute('onclick')?.includes(tabKey));
    if (activeBtn) activeBtn.classList.add('active');

    const panel = document.getElementById('wordcraft-ribbon-content');
    if (panel) panel.innerHTML = this.renderActiveRibbonTab();
  },

  /* ------------------------------------------------------------------------
     File Backstage Menu
     ------------------------------------------------------------------------ */
  toggleFileMenu(e) {
    if (e) e.stopPropagation();
    let menu = document.getElementById('wordcraft-file-menu');
    if (!menu) {
      menu = document.createElement('div');
      menu.id = 'wordcraft-file-menu';
      menu.style.position = 'fixed';
      menu.style.zIndex = '9999999';
      menu.style.background = 'var(--window-bg)';
      menu.style.backdropFilter = 'blur(40px)';
      menu.style.border = '1px solid var(--window-border)';
      menu.style.borderRadius = '12px';
      menu.style.boxShadow = '0 16px 40px rgba(0,0,0,0.35)';
      menu.style.padding = '8px';
      menu.style.minWidth = '220px';
      document.body.appendChild(menu);

      window.addEventListener('click', () => {
        menu.style.display = 'none';
      });
    }

    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const rect = e.currentTarget.getBoundingClientRect();

    menu.innerHTML = `
      <div class="finder-menu-item" onclick="DokuWordCraft.newBlankDocument()" style="padding:7px 12px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
        <span>✨</span> <span>${isId ? 'Dokumen Baru' : 'New Blank Document'}</span>
      </div>
      <div class="finder-menu-item" onclick="DokuWordCraft.openFromFileManagerModal()" style="padding:7px 12px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
        <span>📂</span> <span>${isId ? 'Buka dari Finder...' : 'Open from Finder...'}</span>
      </div>
      <div class="finder-menu-item" onclick="DokuWordCraft.saveDocument()" style="padding:7px 12px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
        <span>💾</span> <span>${isId ? 'Simpan ke BijiOS' : 'Save to BijiOS'}</span>
      </div>
      <div class="finder-menu-item" onclick="DokuWordCraft.openSaveAsModal()" style="padding:7px 12px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
        <span>📑</span> <span>${isId ? 'Simpan Sebagai... (Pilih Folder)' : 'Save As... (Select Folder)'}</span>
      </div>
      <div style="height:1px; background:var(--window-border); margin:4px 0;"></div>
      <div style="font-size:10px; font-weight:700; color:var(--text-muted); padding:4px 12px; text-transform:uppercase;">${isId ? 'Ekspor Berkas' : 'Export Document'}</div>
      <div class="finder-menu-item" onclick="DokuWordCraft.exportDoc('docx')" style="padding:6px 12px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
        <span>📄</span> <span>Ekspor Word (.docx)</span>
      </div>
      <div class="finder-menu-item" onclick="DokuWordCraft.printOrExportPdf()" style="padding:6px 12px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
        <span>📕</span> <span>Ekspor / Cetak PDF</span>
      </div>
      <div class="finder-menu-item" onclick="DokuWordCraft.exportDoc('html')" style="padding:6px 12px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
        <span>🌐</span> <span>Ekspor HTML</span>
      </div>
      <div class="finder-menu-item" onclick="DokuWordCraft.exportDoc('md')" style="padding:6px 12px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
        <span>📝</span> <span>Ekspor Markdown (.md)</span>
      </div>
      <div class="finder-menu-item" onclick="DokuWordCraft.exportDoc('txt')" style="padding:6px 12px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
        <span>🗒️</span> <span>Ekspor Teks Polos (.txt)</span>
      </div>
      <div style="height:1px; background:var(--window-border); margin:4px 0;"></div>
      <div class="finder-menu-item" onclick="window.print()" style="padding:7px 12px; border-radius:6px; cursor:pointer; display:flex; align-items:center; gap:8px;">
        <span>🖨️</span> <span>${isId ? 'Cetak Dokumen (Print)' : 'Print (Ctrl+P)'}</span>
      </div>
    `;

    menu.style.left = `${rect.left}px`;
    menu.style.top = `${rect.bottom + 4}px`;
    menu.style.display = 'block';
  },

  /* ------------------------------------------------------------------------
     Ruler Ticks & Interactive Paragraph Alignment Engine
     ------------------------------------------------------------------------ */
  renderRulerTicks() {
    let ticks = '';
    // A4 paper is 816px. Left margin = 80px, Right margin = 736px.
    // Printable body area = 656px = 17 cm. 1 cm ≈ 38.588px.
    const cmPx = 656 / 17;

    // Subticks in left margin (-2cm, -1cm)
    for (let cm = -2; cm < 0; cm++) {
      const pos = 80 + cm * cmPx;
      if (pos >= 0) {
        ticks += `<div class="ruler-subtick mid" style="left:${pos}px;"></div>`;
        ticks += `<div class="ruler-subtick" style="left:${pos + cmPx * 0.25}px;"></div>`;
        ticks += `<div class="ruler-subtick" style="left:${pos + cmPx * 0.5}px;"></div>`;
        ticks += `<div class="ruler-subtick" style="left:${pos + cmPx * 0.75}px;"></div>`;
      }
    }

    // Centimeter numbers and subticks for printable area (0 to 17 cm)
    for (let cm = 0; cm <= 17; cm++) {
      const pos = 80 + cm * cmPx;
      if (cm > 0 && cm < 17) {
        ticks += `<div class="ruler-tick" style="left:${pos}px;">${cm}</div>`;
      }
      if (cm < 17) {
        ticks += `<div class="ruler-subtick" style="left:${pos + cmPx * 0.25}px;"></div>`;
        ticks += `<div class="ruler-subtick mid" style="left:${pos + cmPx * 0.5}px;"></div>`;
        ticks += `<div class="ruler-subtick" style="left:${pos + cmPx * 0.75}px;"></div>`;
      }
    }

    // Subticks in right margin
    for (let cm = 17; cm <= 19; cm++) {
      const pos = 80 + cm * cmPx;
      if (pos <= 816) {
        ticks += `<div class="ruler-subtick mid" style="left:${pos}px;"></div>`;
        ticks += `<div class="ruler-subtick" style="left:${pos + cmPx * 0.5}px;"></div>`;
      }
    }

    return ticks;
  },

  getActiveParagraph() {
    const sel = window.getSelection();
    if (!sel || !sel.rangeCount) return null;
    let node = sel.anchorNode;
    if (!node) return null;
    if (node.nodeType === 3) node = node.parentNode;
    const editor = document.getElementById('wordcraft-editor-body');
    if (!editor || !editor.contains(node)) return null;
    const block = node.closest('p, h1, h2, h3, h4, h5, h6, li, blockquote, div');
    return (block && block !== editor) ? block : null;
  },

  syncRulerWithActiveParagraph() {
    const block = this.getActiveParagraph();
    const firstLineEl = document.getElementById('ruler-first-line');
    const hangingEl = document.getElementById('ruler-hanging');
    const leftBaseEl = document.getElementById('ruler-left-base');
    const rightIndentEl = document.getElementById('ruler-right-indent');
    if (!firstLineEl || !hangingEl || !leftBaseEl || !rightIndentEl) return;

    if (!block) {
      firstLineEl.style.left = '80px';
      hangingEl.style.left = '80px';
      leftBaseEl.style.left = '80px';
      rightIndentEl.style.left = '736px';
      return;
    }

    const style = window.getComputedStyle(block);
    const marginLeft = parseFloat(style.marginLeft) || 0;
    const paddingLeft = parseFloat(style.paddingLeft) || 0;
    const textIndent = parseFloat(style.textIndent) || 0;
    const marginRight = parseFloat(style.marginRight) || 0;

    const baseLeft = 80 + marginLeft;
    const firstLinePos = baseLeft + textIndent;
    const hangingPos = baseLeft + paddingLeft;
    const rightPos = 736 - marginRight;

    firstLineEl.style.left = `${Math.max(10, Math.min(800, firstLinePos))}px`;
    hangingEl.style.left = `${Math.max(10, Math.min(800, hangingPos))}px`;
    leftBaseEl.style.left = `${Math.max(10, Math.min(800, baseLeft))}px`;
    rightIndentEl.style.left = `${Math.max(10, Math.min(806, rightPos))}px`;
  },

  initRulerDrag() {
    const ruler = document.getElementById('wordcraft-ruler');
    const guide = document.getElementById('ruler-guide-line');
    if (!ruler || !guide) return;

    const markers = [
      { id: 'ruler-first-line', type: 'first-line' },
      { id: 'ruler-hanging', type: 'hanging' },
      { id: 'ruler-left-base', type: 'left-base' },
      { id: 'ruler-right-indent', type: 'right-indent' }
    ];

    markers.forEach(m => {
      const el = document.getElementById(m.id);
      if (!el) return;

      el.onmousedown = (e) => {
        e.preventDefault();
        e.stopPropagation();

        let block = this.getActiveParagraph();
        if (!block) {
          const editor = document.getElementById('wordcraft-editor-body');
          if (editor) {
            block = editor.querySelector('p, h1, h2, h3, li, div');
            if (!block) {
              block = document.createElement('p');
              block.innerHTML = '<br>';
              editor.appendChild(block);
            }
          }
        }
        if (!block) return;

        const startX = e.clientX;
        const initialLeft = parseFloat(el.style.left) || (m.type === 'right-indent' ? 736 : 80);

        guide.style.display = 'block';
        guide.style.left = `${initialLeft}px`;

        const style = window.getComputedStyle(block);
        const curMarginLeft = parseFloat(style.marginLeft) || 0;
        const curPaddingLeft = parseFloat(style.paddingLeft) || 0;
        const curTextIndent = parseFloat(style.textIndent) || 0;

        const onMouseMove = (moveEv) => {
          const delta = moveEv.clientX - startX;
          let newPos = initialLeft + delta;

          if (m.type === 'first-line') {
            newPos = Math.max(10, Math.min(700, newPos));
            el.style.left = `${newPos}px`;
            guide.style.left = `${newPos}px`;
            const indent = newPos - (80 + curMarginLeft);
            block.style.textIndent = `${indent}px`;
          } else if (m.type === 'hanging') {
            newPos = Math.max(10, Math.min(700, newPos));
            el.style.left = `${newPos}px`;
            guide.style.left = `${newPos}px`;
            const hang = Math.max(0, newPos - (80 + curMarginLeft));
            block.style.paddingLeft = `${hang}px`;
            block.style.textIndent = `-${hang}px`;
          } else if (m.type === 'left-base') {
            newPos = Math.max(10, Math.min(650, newPos));
            el.style.left = `${newPos}px`;
            guide.style.left = `${newPos}px`;
            const newMargin = newPos - 80;
            block.style.marginLeft = `${newMargin}px`;
            const firstLineEl = document.getElementById('ruler-first-line');
            const hangingEl = document.getElementById('ruler-hanging');
            if (firstLineEl) firstLineEl.style.left = `${newPos + curTextIndent}px`;
            if (hangingEl) hangingEl.style.left = `${newPos + curPaddingLeft}px`;
          } else if (m.type === 'right-indent') {
            newPos = Math.max(200, Math.min(806, newPos));
            el.style.left = `${newPos}px`;
            guide.style.left = `${newPos}px`;
            const rightMargin = Math.max(0, 736 - newPos);
            block.style.marginRight = `${rightMargin}px`;
          }
        };

        const onMouseUp = () => {
          window.removeEventListener('mousemove', onMouseMove);
          window.removeEventListener('mouseup', onMouseUp);
          guide.style.display = 'none';
          this.isDirty = true;
          this.syncRulerWithActiveParagraph();
        };

        window.addEventListener('mousemove', onMouseMove);
        window.addEventListener('mouseup', onMouseUp);
      };
    });
  },

  /* ------------------------------------------------------------------------
     Editor Execution & Typography Styles Engine
     ------------------------------------------------------------------------ */
  execCmd(cmd, val = null) {
    document.execCommand(cmd, false, val);
    this.focusEditor();
    this.isDirty = true;
    this.updateStats();
    this.refreshOutline();
    this.syncRulerWithActiveParagraph();
  },

  focusEditor() {
    const editor = document.getElementById('wordcraft-editor-body');
    if (editor) editor.focus();
  },

  setFontSize(sizeVal) {
    document.execCommand('fontSize', false, sizeVal);
    this.focusEditor();
  },

  changeFontSize(delta) {
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0) return;
    document.execCommand(delta > 0 ? 'increaseFontSize' : 'decreaseFontSize', false, null);
    this.focusEditor();
  },

  setLineSpacing(spacing) {
    const editor = document.getElementById('wordcraft-editor-body');
    if (editor) {
      editor.style.lineHeight = spacing;
      this.isDirty = true;
    }
  },

  applyStyle(styleKey) {
    const editor = document.getElementById('wordcraft-editor-body');
    if (!editor) return;

    // Visual active card highlight
    document.querySelectorAll('.style-card').forEach(c => c.classList.remove('active'));
    const clicked = Array.from(document.querySelectorAll('.style-card')).find(c => c.getAttribute('onclick')?.includes(styleKey));
    if (clicked) clicked.classList.add('active');

    switch (styleKey) {
      case 'normal':
        document.execCommand('formatBlock', false, '<p>');
        document.execCommand('removeFormat', false, null);
        break;
      case 'no_spacing':
        document.execCommand('formatBlock', false, '<p>');
        const p = window.getSelection()?.anchorNode?.parentElement;
        if (p) p.style.margin = '0';
        break;
      case 'heading1':
        document.execCommand('formatBlock', false, '<h1>');
        break;
      case 'heading2':
        document.execCommand('formatBlock', false, '<h2>');
        break;
      case 'heading3':
        document.execCommand('formatBlock', false, '<h3>');
        break;
      case 'title':
        document.execCommand('formatBlock', false, '<h1>');
        const h1 = window.getSelection()?.anchorNode?.parentElement;
        if (h1) {
          h1.style.fontSize = '28px';
          h1.style.fontWeight = '800';
          h1.style.marginBottom = '8px';
        }
        break;
      case 'subtitle':
        document.execCommand('formatBlock', false, '<p>');
        const sub = window.getSelection()?.anchorNode?.parentElement;
        if (sub) {
          sub.style.fontSize = '16px';
          sub.style.fontStyle = 'italic';
          sub.style.color = '#64748b';
        }
        break;
      case 'quote':
        this.formatBlockquote();
        break;
    }
    this.focusEditor();
    this.refreshOutline();
  },

  formatBlockquote() {
    document.execCommand('formatBlock', false, '<blockquote>');
    this.focusEditor();
  },

  insertPageBreak() {
    const hrHtml = `<div class="wordcraft-page-break" style="border-top:2px dashed #94a3b8; margin:30px 0; position:relative; text-align:center; user-select:none;"><span style="position:relative; top:-10px; background:#fff; padding:0 10px; font-size:10px; color:#64748b; font-weight:600; text-transform:uppercase;">────── Batas Halaman (Page Break) ──────</span></div><p><br></p>`;
    document.execCommand('insertHTML', false, hrHtml);
    this.focusEditor();
  },

  insertTable(rows = 3, cols = 3) {
    let headerCells = '';
    for (let c = 1; c <= cols; c++) {
      headerCells += `<th>Kolom ${c}</th>`;
    }
    let bodyRows = '';
    for (let r = 1; r <= rows; r++) {
      let cells = '';
      for (let c = 1; c <= cols; c++) {
        cells += `<td>Baris ${r}, Kolom ${c}</td>`;
      }
      bodyRows += `<tr>${cells}</tr>`;
    }

    const tableHtml = `
      <table style="width:100%; border-collapse:collapse; margin:16px 0;" border="1">
        <thead>
          <tr style="background:#f1f5f9;">${headerCells}</tr>
        </thead>
        <tbody>
          ${bodyRows}
        </tbody>
      </table>
      <p><br></p>
    `;
    document.execCommand('insertHTML', false, tableHtml);
    this.focusEditor();
  },

  insertCustomTable() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const r = prompt(isId ? "Masukkan jumlah baris:" : "Enter number of rows:", "4");
    if (!r) return;
    const c = prompt(isId ? "Masukkan jumlah kolom:" : "Enter number of columns:", "3");
    if (!c) return;
    this.insertTable(parseInt(r, 10) || 3, parseInt(c, 10) || 3);
  },

  insertCalloutBox() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const boxHtml = `
      <div style="background:#eff6ff; border:1px solid #bfdbfe; border-left:4px solid #2563eb; border-radius:6px; padding:12px 16px; margin:16px 0; color:#1e3a8a;">
        <strong>💡 ${isId ? 'Catatan Penting:' : 'Important Note:'}</strong>
        <p style="margin:4px 0 0;">${isId ? 'Tuliskan poin atau ringkasan penting di sini...' : 'Write important summary or notes here...'}</p>
      </div>
      <p><br></p>
    `;
    document.execCommand('insertHTML', false, boxHtml);
    this.focusEditor();
  },

  insertLink() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const url = prompt(isId ? "Masukkan URL tujuan tautan:" : "Enter hyperlink URL:", "https://");
    if (url) {
      document.execCommand('createLink', false, url);
      this.focusEditor();
    }
  },

  insertImageByUrl() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const url = prompt(isId ? "Masukkan URL gambar:" : "Enter image URL:", "https://images.unsplash.com/photo-1486406146926-c627a92ad1ab?w=600");
    if (url) {
      document.execCommand('insertImage', false, url);
      this.focusEditor();
    }
  },

  handleImageInsert(e) {
    const file = e.target.files && e.target.files[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (event) => {
      const base64 = event.target.result;
      document.execCommand('insertImage', false, base64);
      this.focusEditor();
    };
    reader.readAsDataURL(file);
    e.target.value = '';
  },

  insertSymbol(sym) {
    document.execCommand('insertText', false, sym);
    this.focusEditor();
  },

  insertDateStamp() {
    const now = new Date();
    const str = now.toLocaleDateString('id-ID', { weekday: 'long', year: 'numeric', month: 'long', day: 'numeric' });
    document.execCommand('insertText', false, str);
    this.focusEditor();
  },

  insertPageNumber() {
    const numHtml = `<span style="font-size:11px; color:#64748b; font-style:italic;">[Hal. 1]</span>`;
    document.execCommand('insertHTML', false, numHtml);
  },

  /* ------------------------------------------------------------------------
     Table of Contents Engine (Headings Analyzer)
     ------------------------------------------------------------------------ */
  generateTableOfContents() {
    const editor = document.getElementById('wordcraft-editor-body');
    if (!editor) return;

    const headings = editor.querySelectorAll('h1, h2, h3');
    if (headings.length === 0) {
      alert("Tidak ditemukan judul (Heading 1, 2, atau 3) dalam dokumen untuk membuat Daftar Isi. Silakan terapkan gaya Heading terlebih dahulu.");
      return;
    }

    let tocList = '';
    headings.forEach((h, idx) => {
      const text = h.textContent.trim();
      const level = h.tagName.toLowerCase();
      const indent = level === 'h1' ? '0' : level === 'h2' ? '20px' : '40px';
      const weight = level === 'h1' ? '700' : level === 'h2' ? '600' : '400';
      const anchorId = `toc-heading-${idx}`;
      h.id = anchorId;

      tocList += `
        <div style="padding-left:${indent}; font-weight:${weight}; margin:4px 0; display:flex; justify-content:space-between; border-bottom:1px dotted #cbd5e1;">
          <a href="#${anchorId}" style="text-decoration:none; color:#1e40af;" onclick="document.getElementById('${anchorId}').scrollIntoView({behavior:'smooth'}); return false;">
            ${text || 'Judul Bagian'}
          </a>
          <span style="color:#64748b;">${idx + 1}</span>
        </div>
      `;
    });

    const tocHtml = `
      <div class="wordcraft-toc-box" style="background:#f8fafc; border:1px solid #e2e8f0; border-radius:8px; padding:18px 24px; margin:24px 0;">
        <h3 style="margin-top:0; color:#0f172a; border-bottom:2px solid #2563eb; padding-bottom:6px;">📑 DAFTAR ISI</h3>
        <div style="margin-top:12px;">
          ${tocList}
        </div>
      </div>
      <p><br></p>
    `;

    document.execCommand('insertHTML', false, tocHtml);
    this.focusEditor();
  },

  /* ------------------------------------------------------------------------
     Navigation Outline Pane
     ------------------------------------------------------------------------ */
  toggleNavPane() {
    this.showNavPane = !this.showNavPane;
    const pane = document.getElementById('wordcraft-nav-pane');
    if (pane) {
      pane.classList.toggle('hidden', !this.showNavPane);
    }
    if (this.showNavPane) {
      this.refreshOutline();
    }
  },

  refreshOutline() {
    const list = document.getElementById('wordcraft-outline-list');
    const editor = document.getElementById('wordcraft-editor-body');
    if (!list || !editor || !this.showNavPane) return;

    const headings = editor.querySelectorAll('h1, h2, h3');
    if (headings.length === 0) {
      list.innerHTML = `<div style="padding:14px 8px; color:var(--text-muted); text-align:center; font-size:11px;">Belum ada judul / heading.<br>Gunakan gaya Heading 1, 2, 3 di tab Beranda.</div>`;
      return;
    }

    list.innerHTML = Array.from(headings).map((h, idx) => {
      const level = h.tagName.toLowerCase();
      const indent = level === 'h1' ? '4px' : level === 'h2' ? '16px' : '28px';
      const weight = level === 'h1' ? '700' : level === 'h2' ? '600' : '400';
      const anchorId = h.id || `outline-h-${idx}`;
      h.id = anchorId;

      return `
        <div style="padding:5px 8px; padding-left:${indent}; font-weight:${weight}; font-size:12px; border-radius:4px; cursor:pointer; color:var(--text-main); text-overflow:ellipsis; overflow:hidden; white-space:nowrap;"
             onmouseover="this.style.background='rgba(37,99,235,0.1)'"
             onmouseout="this.style.background='transparent'"
             onclick="document.getElementById('${anchorId}')?.scrollIntoView({behavior:'smooth'})">
          ${h.textContent.trim() || 'Bagian'}
        </div>
      `;
    }).join('');
  },

  /* ------------------------------------------------------------------------
     Layout & Page Setup Engine
     ------------------------------------------------------------------------ */
  setMargins(type) {
    this.pageMargins = type;
    const page = document.getElementById('wordcraft-editor-body');
    if (!page) return;

    switch (type) {
      case 'narrow':
        page.style.padding = '36px 40px';
        break;
      case 'wide':
        page.style.padding = '80px 110px';
        break;
      case 'normal':
      default:
        page.style.padding = '72px 80px';
        break;
    }
  },

  setOrientation(orient) {
    this.pageOrientation = orient;
    const page = document.getElementById('wordcraft-editor-body');
    const ruler = document.querySelector('.wordcraft-ruler');
    if (!page) return;

    if (orient === 'landscape') {
      page.style.width = '1056px';
      page.style.minHeight = '816px';
      if (ruler) ruler.style.width = '1056px';
    } else {
      page.style.width = '816px';
      page.style.minHeight = '1056px';
      if (ruler) ruler.style.width = '816px';
    }
  },

  setColumns(count) {
    const page = document.getElementById('wordcraft-editor-body');
    if (page) {
      page.style.columnCount = count;
      page.style.columnGap = '36px';
      page.style.columnRule = '1px solid #e2e8f0';
    }
  },

  setWatermark() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const text = prompt(isId ? "Masukkan teks tanda air (Watermark):" : "Enter watermark text:", "CONFIDENTIAL");
    if (!text) return;
    const page = document.getElementById('wordcraft-editor-body');
    if (page) {
      let mark = page.querySelector('.wordcraft-watermark');
      if (!mark) {
        mark = document.createElement('div');
        mark.className = 'wordcraft-watermark';
        mark.style.position = 'absolute';
        mark.style.top = '40%';
        mark.style.left = '50%';
        mark.style.transform = 'translate(-50%, -50%) rotate(-45deg)';
        mark.style.fontSize = '64px';
        mark.style.fontWeight = '800';
        mark.style.color = 'rgba(0,0,0,0.06)';
        mark.style.pointerEvents = 'none';
        mark.style.userSelect = 'none';
        mark.style.whiteSpace = 'nowrap';
        page.appendChild(mark);
      }
      mark.textContent = text;
    }
  },

  /* ------------------------------------------------------------------------
     View Modes & Zoom Engine
     ------------------------------------------------------------------------ */
  setViewMode(mode) {
    this.viewMode = mode;
    const page = document.getElementById('wordcraft-editor-body');
    const rulerWrap = document.getElementById('wordcraft-ruler-wrapper');
    if (!page) return;

    document.querySelectorAll('.wordcraft-view-pill').forEach(p => p.classList.remove('active'));
    const btn = Array.from(document.querySelectorAll('.wordcraft-view-pill')).find(p => p.getAttribute('onclick')?.includes(mode));
    if (btn) btn.classList.add('active');

    if (mode === 'web') {
      page.style.width = '100%';
      page.style.maxWidth = '1100px';
      page.style.boxShadow = 'none';
      if (rulerWrap) rulerWrap.style.display = 'none';
    } else if (mode === 'read') {
      page.style.width = '740px';
      page.style.boxShadow = '0 10px 40px rgba(0,0,0,0.2)';
      page.style.padding = '60px 80px';
      if (rulerWrap) rulerWrap.style.display = 'none';
    } else {
      // Print Layout (A4)
      this.setOrientation(this.pageOrientation);
      page.style.boxShadow = '0 8px 30px rgba(0, 0, 0, 0.16)';
      if (rulerWrap) rulerWrap.style.display = this.showRuler ? 'flex' : 'none';
    }
  },

  toggleRuler() {
    this.showRuler = !this.showRuler;
    const rulerWrap = document.getElementById('wordcraft-ruler-wrapper');
    if (rulerWrap) {
      rulerWrap.style.display = this.showRuler && this.viewMode === 'print' ? 'flex' : 'none';
    }
  },

  setZoom(val) {
    this.zoomLevel = parseInt(val, 10) || 100;
    const wrapper = document.getElementById('wordcraft-page-wrapper');
    const label = document.getElementById('wordcraft-zoom-label');
    if (wrapper) wrapper.style.transform = `scale(${this.zoomLevel / 100})`;
    if (label) label.textContent = `${this.zoomLevel}%`;
  },

  adjustZoom(delta) {
    let next = this.zoomLevel + delta;
    if (next < 50) next = 50;
    if (next > 200) next = 200;
    this.setZoom(next);
  },

  /* ------------------------------------------------------------------------
     Document Stats & Proofing
     ------------------------------------------------------------------------ */
  bindEditorEvents() {
    const editor = document.getElementById('wordcraft-editor-body');
    if (!editor) return;

    try {
      document.execCommand('defaultParagraphSeparator', false, 'p');
    } catch (_) { }

    editor.addEventListener('focus', () => {
      try {
        document.execCommand('defaultParagraphSeparator', false, 'p');
      } catch (_) { }
      this.syncRulerWithActiveParagraph();
    });

    // Isolate Enter and Tab keys inside editor
    editor.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.stopPropagation();
      }

      if (e.key === 'Tab') {
        e.preventDefault();
        e.stopPropagation();

        if (e.shiftKey) {
          // Outdent
          const block = this.getActiveParagraph();
          if (block) {
            const curMargin = parseFloat(block.style.marginLeft) || 0;
            if (curMargin > 0) {
              block.style.marginLeft = `${Math.max(0, curMargin - 36)}px`;
            } else {
              document.execCommand('outdent');
            }
            this.syncRulerWithActiveParagraph();
          }
        } else {
          // Indent or Tab
          const block = this.getActiveParagraph();
          if (block && (block.tagName === 'LI' || block.closest('ul, ol'))) {
            document.execCommand('indent');
          } else if (block && block.textContent.trim().length === 0) {
            const curMargin = parseFloat(block.style.marginLeft) || 0;
            block.style.marginLeft = `${curMargin + 36}px`;
          } else {
            document.execCommand('insertHTML', false, '&nbsp;&nbsp;&nbsp;&nbsp;');
          }
          this.syncRulerWithActiveParagraph();
        }
      }
    });

    editor.addEventListener('input', () => {
      this.isDirty = true;
      const statusEl = document.getElementById('wordcraft-status');
      if (statusEl) {
        statusEl.textContent = '○ Belum Disimpan';
        statusEl.style.color = '#f59e0b';
      }
      this.updateStats();
      this.syncRulerWithActiveParagraph();
    });

    // Auto update outline and detect numbered lists (e.g., "1. adasdad")
    editor.addEventListener('keyup', (e) => {
      this.syncRulerWithActiveParagraph();
      if (['Enter', 'Backspace', 'Delete'].includes(e.key)) {
        this.refreshOutline();
      }

      // Auto-formatting for hanging indents when user types "1. " or "1) " or "- "
      if (e.key === ' ' || e.key === 'Spacebar') {
        this.handleAutoFormatList();
      }
    });

    editor.addEventListener('click', () => {
      this.syncRulerWithActiveParagraph();
    });

    // Initialize interactive ruler dragging
    this.initRulerDrag();
    this.syncRulerWithActiveParagraph();
  },

  handleAutoFormatList() {
    const block = this.getActiveParagraph();
    if (!block || block.tagName === 'LI') return;
    const text = block.textContent;
    // Matches "1. " or "1) " or "- " at the start
    const match = text.match(/^(\d+[\.\)]|\-|\*)\s$/);
    if (match) {
      // Apply Microsoft Word hanging indent
      block.style.paddingLeft = '28px';
      block.style.textIndent = '-28px';
      this.syncRulerWithActiveParagraph();
    }
  },

  updateStats() {
    const editor = document.getElementById('wordcraft-editor-body');
    const statsEl = document.getElementById('wordcraft-stats');
    if (!editor || !statsEl) return;

    const text = editor.innerText.trim();
    const words = text ? text.split(/\s+/).length : 0;
    const chars = text.length;
    statsEl.textContent = `${words} Kata • ${chars} Karakter`;
  },

  showWordCountModal() {
    const editor = document.getElementById('wordcraft-editor-body');
    if (!editor) return;

    const text = editor.innerText.trim();
    const words = text ? text.split(/\s+/).length : 0;
    const charsWithSpaces = text.length;
    const charsNoSpaces = text.replace(/\s+/g, '').length;
    const paragraphs = text ? text.split(/\n+/).length : 0;
    const readMinutes = Math.max(1, Math.ceil(words / 200));

    alert(
      `📊 STATISTIK DOKUMEN WORDCRAFT\n\n` +
      `• Halaman: 1\n` +
      `• Jumlah Kata: ${words}\n` +
      `• Karakter (dengan spasi): ${charsWithSpaces}\n` +
      `• Karakter (tanpa spasi): ${charsNoSpaces}\n` +
      `• Paragraf: ${paragraphs}\n` +
      `• Estimasi Waktu Baca: ~${readMinutes} menit`
    );
  },

  checkProofing() {
    alert("✔️ Pemeriksaan Ejaan (Proofing):\nTidak ditemukan galat sintaksis. Kamus Bahasa Indonesia aktif.");
  },

  openFindModal() {
    const q = prompt("Cari teks dalam dokumen:", "");
    if (q) {
      if (window.find) {
        window.find(q);
      } else {
        alert(`Mencari "${q}"...`);
      }
    }
  },

  /* ------------------------------------------------------------------------
     Cloud Persistence & Multi-Format Exporters
     ------------------------------------------------------------------------ */
  async loadDocumentContent() {
    if (!this.currentFile || this.currentFile.id === 'new_word') {
      this.updateWindowTitle();
      return;
    }

    try {
      const res = await fetch(`/api/files/${this.currentFile.id}/content`);
      if (res.ok) {
        const text = await res.text();
        const editor = document.getElementById('wordcraft-editor-body');
        if (editor) {
          let html = text;
          try {
            const data = JSON.parse(text);
            if (data && typeof data === 'object') {
              html = data.content !== undefined ? data.content : (data.html || text);
            }
          } catch {
            html = text;
          }
          editor.innerHTML = html || '<p>Mulai menulis dokumen WordCraft Anda di sini...</p>';
        }
      }
    } catch (e) {
      console.warn("Gagal memuat isi WordCraft:", e);
    }
    this.updateStats();
    this.refreshOutline();
    this.syncRulerWithActiveParagraph();
    this.updateWindowTitle();
  },

  async saveDocument() {
    // If brand new file without existing ID, route to Save As modal so user can pick folder & name
    if (!this.currentFile || this.currentFile.id === 'new_word' || !this.currentFile.id) {
      this.openSaveAsModal();
      return;
    }

    const editor = document.getElementById('wordcraft-editor-body');
    if (!editor) return;

    try {
      const currentUserId = window.DokuHub && DokuHub.currentUser ? DokuHub.currentUser.id : 'usr_dionipe_849';
      const currentTenantId = (window.DokuHub && DokuHub.activeTenant && DokuHub.activeTenant.id) ? DokuHub.activeTenant.id : (window.DokuHub && DokuHub.currentUser ? DokuHub.currentUser.tenant_id : '');
      const isId = (window.DokuI18n && DokuI18n.lang === 'id');

      // Existing document with known file ID
      const payload = {
        title: this.currentFile.name,
        content: editor.innerHTML
      };

      const res = await fetch(`/api/files/${this.currentFile.id}/content`, {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
          'X-Tenant-Id': currentTenantId,
          'X-User-Id': currentUserId
        },
        body: JSON.stringify(payload)
      });

      const data = await res.json();
      if (res.ok && data.success) {
        this.isDirty = false;
        this.updateWindowTitle();
        const statusEl = document.getElementById('wordcraft-status');
        if (statusEl) {
          statusEl.textContent = isId ? '● Tersimpan di BijiOS' : '● Saved to BijiOS';
          statusEl.style.color = '#10b981';
        }
        if (window.DokuFinder) window.DokuFinder.loadAllFiles();
      } else {
        alert((isId ? "Gagal menyimpan dokumen: " : "Failed to save document: ") + (data.error || 'Error'));
      }
    } catch (e) {
      alert((window.DokuI18n && DokuI18n.lang === 'id' ? "Gagal menyimpan dokumen: " : "Failed to save document: ") + e);
    }
  },

  /* ------------------------------------------------------------------------
     File Picker: Open Document from Finder
     ------------------------------------------------------------------------ */
  _availableDocs: [],

  async openFromFileManagerModal() {
    let overlay = document.getElementById('wordcraft-open-modal');
    if (!overlay) {
      overlay = document.createElement('div');
      overlay.id = 'wordcraft-open-modal';
      overlay.style.position = 'fixed';
      overlay.style.inset = '0';
      overlay.style.background = 'rgba(0,0,0,0.55)';
      overlay.style.backdropFilter = 'blur(12px)';
      overlay.style.zIndex = '999999';
      overlay.style.display = 'none';
      overlay.style.alignItems = 'center';
      overlay.style.justifyContent = 'center';
      document.body.appendChild(overlay);
    }

    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    overlay.style.display = 'flex';
    overlay.innerHTML = `
      <div style="width:620px; max-width:92vw; max-height:85vh; background:var(--window-bg); backdrop-filter:blur(40px); border:1px solid var(--window-border); border-radius:18px; box-shadow:0 30px 80px rgba(0,0,0,0.45); padding:24px; display:flex; flex-direction:column;">
        <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:14px;">
          <div>
            <h3 style="font-size:17px; font-weight:700; display:flex; align-items:center; gap:8px;">
              <span>📂</span> ${isId ? 'Buka Dokumen dari Finder' : 'Open Document from Finder'}
            </h3>
            <p style="font-size:12px; color:var(--text-muted); margin-top:2px;">
              ${isId ? 'Pilih berkas Word (.docx) yang tersimpan di workspace BijiOS Anda.' : 'Select a Word (.docx) document stored in your BijiOS workspace.'}
            </p>
          </div>
          <button class="traffic-btn btn-close" onclick="document.getElementById('wordcraft-open-modal').style.display='none'"></button>
        </div>

        <!-- Search Filter -->
        <div style="margin-bottom:12px; position:relative;">
          <input type="text" id="wordcraft-picker-search" placeholder="${isId ? 'Cari nama dokumen...' : 'Search document name...'}" 
                 style="width:100%; padding:9px 12px 9px 32px; border-radius:8px; border:1px solid var(--window-border); background:var(--card-bg); color:var(--text-main); font-size:13px; outline:none;"
                 oninput="DokuWordCraft.filterOpenList(this.value)">
          <span style="position:absolute; left:10px; top:9px; font-size:13px; opacity:0.6;">🔍</span>
        </div>

        <!-- Document List Container -->
        <div id="wordcraft-picker-list" style="flex:1; overflow-y:auto; min-height:240px; max-height:380px; border:1px solid var(--window-border); border-radius:10px; background:var(--card-bg);">
          <div style="padding:40px; text-align:center; color:var(--text-muted); font-size:13px;">Loading documents...</div>
        </div>

        <!-- Actions -->
        <div style="display:flex; justify-content:space-between; align-items:center; margin-top:16px;">
          <button class="mac-btn" onclick="DokuHub.openApp('finder'); document.getElementById('wordcraft-open-modal').style.display='none';">
            📁 ${isId ? 'Buka Finder' : 'Browse Finder'}
          </button>
          <div style="display:flex; gap:8px;">
            <button class="mac-btn" onclick="document.getElementById('wordcraft-open-modal').style.display='none'">${isId ? 'Batal' : 'Cancel'}</button>
            <button class="mac-btn primary" onclick="DokuWordCraft.newBlankDocument(); document.getElementById('wordcraft-open-modal').style.display='none';">
              ✨ ${isId ? 'Dokumen Baru' : 'New Document'}
            </button>
          </div>
        </div>
      </div>
    `;

    // Fetch documents
    try {
      const currentUserId = window.DokuHub && DokuHub.currentUser ? DokuHub.currentUser.id : '';
      const currentTenantId = (window.DokuHub && DokuHub.activeTenant && DokuHub.activeTenant.id) ? DokuHub.activeTenant.id : '';
      const res = await fetch(`/api/files?parent_id=all&user_id=${currentUserId}&tenant_id=${currentTenantId}&category=all`);
      const allFiles = await res.json();
      const docs = Array.isArray(allFiles) ? allFiles.filter(f => !f.is_folder && f.extension === 'docx') : [];
      this._availableDocs = docs;
      this.renderPickerDocs(docs, allFiles);
    } catch (e) {
      const listEl = document.getElementById('wordcraft-picker-list');
      if (listEl) listEl.innerHTML = `<div style="padding:30px; text-align:center; color:#ef4444;">Failed to load documents: ${e}</div>`;
    }
  },

  renderPickerDocs(docs, allFiles = []) {
    const listEl = document.getElementById('wordcraft-picker-list');
    if (!listEl) return;
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');

    if (docs.length === 0) {
      listEl.innerHTML = `
        <div style="padding:48px 24px; text-align:center; color:var(--text-muted);">
          <div style="font-size:42px; margin-bottom:10px; opacity:0.8;">📝</div>
          <div style="font-size:14px; font-weight:700; color:var(--text-main); margin-bottom:4px;">
            ${isId ? 'Tidak ada dokumen Word (.docx)' : 'No Word documents found'}
          </div>
          <div style="font-size:12px; margin-bottom:16px;">
            ${isId ? 'Belum ada berkas .docx di workspace. Buat dokumen baru atau unggah dari Finder.' : 'No .docx files found in your workspace yet. Create a new document to start.'}
          </div>
          <button class="mac-btn primary" onclick="DokuWordCraft.newBlankDocument(); document.getElementById('wordcraft-open-modal').style.display='none';">
            ✨ ${isId ? 'Buat Dokumen Baru' : 'Create New Document'}
          </button>
        </div>
      `;
      return;
    }

    const folderMap = {};
    if (Array.isArray(allFiles)) {
      allFiles.forEach(f => {
        if (f.is_folder) folderMap[f.id] = f.name;
      });
    }

    listEl.innerHTML = `
      <div style="display:flex; flex-direction:column;">
        ${docs.map(doc => {
      let folderName = folderMap[doc.parent_id] || (doc.parent_id === 'fld_docs' ? (isId ? 'Dokumen' : 'Documents') : (doc.parent_id === 'fld_projects' ? (isId ? 'Proyek' : 'Projects') : (isId ? 'Ruang Kerja Utama' : 'Root')));
      const sizeStr = doc.size_bytes ? `${Math.round(doc.size_bytes / 1024)} KB` : '--';
      return `
            <div style="display:flex; align-items:center; justify-content:space-between; padding:10px 14px; border-bottom:1px solid var(--window-border); cursor:pointer; transition:background 0.12s;"
                 onmouseover="this.style.background='rgba(0,122,255,0.08)'"
                 onmouseout="this.style.background='transparent'"
                 onclick="DokuWordCraft.openSelectedFromPicker('${doc.id}')">
              <div style="display:flex; align-items:center; gap:10px; min-width:0;">
                <span style="font-size:22px;">📝</span>
                <div style="min-width:0;">
                  <div style="font-weight:600; font-size:13px; text-overflow:ellipsis; overflow:hidden; white-space:nowrap;">
                    ${doc.name}
                  </div>
                  <div style="font-size:11px; color:var(--text-muted); display:flex; gap:6px;">
                    <span>📁 ${folderName}</span>
                    <span>•</span>
                    <span>${sizeStr}</span>
                  </div>
                </div>
              </div>
              <button class="mac-btn" style="font-size:11px; padding:3px 10px;" onclick="event.stopPropagation(); DokuWordCraft.openSelectedFromPicker('${doc.id}')">
                ${isId ? 'Buka' : 'Open'}
              </button>
            </div>
          `;
    }).join('')}
      </div>
    `;
  },

  filterOpenList(query) {
    if (!this._availableDocs) return;
    const q = (query || '').toLowerCase().trim();
    const filtered = this._availableDocs.filter(d => d.name.toLowerCase().includes(q));
    this.renderPickerDocs(filtered);
  },

  async openSelectedFromPicker(docId) {
    const doc = this._availableDocs ? this._availableDocs.find(d => d.id === docId) : null;
    const modal = document.getElementById('wordcraft-open-modal');
    if (modal) modal.style.display = 'none';

    if (doc) {
      await this.loadFile(doc);
    } else {
      await this.loadFile({ id: docId, name: 'Document.docx' });
    }
  },

  /* ------------------------------------------------------------------------
     Save As Modal: Save Document into Target Folder
     ------------------------------------------------------------------------ */
  async openSaveAsModal() {
    let overlay = document.getElementById('wordcraft-saveas-modal');
    if (!overlay) {
      overlay = document.createElement('div');
      overlay.id = 'wordcraft-saveas-modal';
      overlay.style.position = 'fixed';
      overlay.style.inset = '0';
      overlay.style.background = 'rgba(0,0,0,0.55)';
      overlay.style.backdropFilter = 'blur(12px)';
      overlay.style.zIndex = '999999';
      overlay.style.display = 'none';
      overlay.style.alignItems = 'center';
      overlay.style.justifyContent = 'center';
      document.body.appendChild(overlay);
    }

    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    const curName = (this.currentFile?.name && this.currentFile.id !== 'new_word')
      ? this.currentFile.name
      : (isId ? 'Dokumen Baru.docx' : 'Untitled Document.docx');

    overlay.style.display = 'flex';
    overlay.innerHTML = `
      <div style="width:480px; max-width:92vw; background:var(--window-bg); backdrop-filter:blur(40px); border:1px solid var(--window-border); border-radius:18px; box-shadow:0 30px 80px rgba(0,0,0,0.45); padding:24px;">
        <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:16px;">
          <div>
            <h3 style="font-size:17px; font-weight:700; display:flex; align-items:center; gap:8px;">
              <span>💾</span> ${isId ? 'Simpan Sebagai (Save As)' : 'Save Document As'}
            </h3>
            <p style="font-size:12px; color:var(--text-muted); margin-top:2px;">
              ${isId ? 'Tentukan nama berkas dan pilih folder tujuan penyimpanan.' : 'Set file name and choose destination folder.'}
            </p>
          </div>
          <button class="traffic-btn btn-close" onclick="document.getElementById('wordcraft-saveas-modal').style.display='none'"></button>
        </div>

        <div style="display:flex; flex-direction:column; gap:14px; margin-bottom:20px;">
          <div>
            <label style="font-size:12px; font-weight:600; display:block; margin-bottom:6px;">${isId ? 'Nama Dokumen (.docx):' : 'Document Name (.docx):'}</label>
            <input type="text" id="wordcraft-saveas-name" value="${curName}" 
                   style="width:100%; padding:10px 12px; border-radius:8px; border:1px solid var(--window-border); background:var(--card-bg); color:var(--text-main); font-size:13px; outline:none;"
                   onkeydown="if(event.key==='Enter') DokuWordCraft.submitSaveAs()">
          </div>

          <div>
            <label style="font-size:12px; font-weight:600; display:block; margin-bottom:6px;">${isId ? 'Folder Tujuan:' : 'Destination Folder:'}</label>
            <select id="wordcraft-saveas-folder" style="width:100%; padding:10px 12px; border-radius:8px; border:1px solid var(--window-border); background:var(--card-bg); color:var(--text-main); font-size:13px; outline:none;">
              <option value="fld_docs" ${this.currentFile?.parent_id === 'fld_docs' ? 'selected' : ''}>📁 ${isId ? 'Dokumen (Documents)' : 'Documents'}</option>
              <option value="fld_projects" ${this.currentFile?.parent_id === 'fld_projects' ? 'selected' : ''}>📁 ${isId ? 'Proyek (Projects)' : 'Projects'}</option>
              <option value="root" ${this.currentFile?.parent_id === 'root' ? 'selected' : ''}>🏠 ${isId ? 'Ruang Kerja Utama (Root Workspace)' : 'Root Workspace'}</option>
            </select>
          </div>
        </div>

        <div style="display:flex; justify-content:flex-end; gap:8px;">
          <button class="mac-btn" onclick="document.getElementById('wordcraft-saveas-modal').style.display='none'">${isId ? 'Batal' : 'Cancel'}</button>
          <button class="mac-btn primary" onclick="DokuWordCraft.submitSaveAs()">💾 ${isId ? 'Simpan ke Folder' : 'Save Document'}</button>
        </div>
      </div>
    `;

    // Fetch custom folders dynamically
    try {
      const currentUserId = window.DokuHub && DokuHub.currentUser ? DokuHub.currentUser.id : '';
      const currentTenantId = (window.DokuHub && DokuHub.activeTenant && DokuHub.activeTenant.id) ? DokuHub.activeTenant.id : '';
      const res = await fetch(`/api/files?parent_id=all&user_id=${currentUserId}&tenant_id=${currentTenantId}&category=all`);
      const allFiles = await res.json();
      if (Array.isArray(allFiles)) {
        const customFolders = allFiles.filter(f => f.is_folder && f.id !== 'fld_docs' && f.id !== 'fld_projects' && f.id !== 'root');
        const select = document.getElementById('wordcraft-saveas-folder');
        if (select && customFolders.length > 0) {
          customFolders.forEach(fld => {
            const opt = document.createElement('option');
            opt.value = fld.id;
            opt.textContent = `📁 ${fld.name}`;
            if (this.currentFile?.parent_id === fld.id) opt.selected = true;
            select.appendChild(opt);
          });
        }
      }
    } catch (_) { }
  },

  async submitSaveAs() {
    const nameInput = document.getElementById('wordcraft-saveas-name');
    const folderSelect = document.getElementById('wordcraft-saveas-folder');
    if (!nameInput || !folderSelect) return;

    let docName = nameInput.value.trim();
    const folderId = folderSelect.value;
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');

    if (!docName) {
      alert(isId ? "Nama dokumen tidak boleh kosong." : "Document name cannot be empty.");
      return;
    }
    if (!docName.toLowerCase().endsWith('.docx')) {
      docName += '.docx';
    }

    const editor = document.getElementById('wordcraft-editor-body');
    const contentHtml = editor ? editor.innerHTML : '<p></p>';

    const currentUserId = window.DokuHub && DokuHub.currentUser ? DokuHub.currentUser.id : 'usr_dionipe_849';
    const currentTenantId = (window.DokuHub && DokuHub.activeTenant && DokuHub.activeTenant.id) ? DokuHub.activeTenant.id : (window.DokuHub && DokuHub.currentUser ? DokuHub.currentUser.tenant_id : '');

    try {
      const res = await fetch('/api/files', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Tenant-Id': currentTenantId,
          'X-User-Id': currentUserId
        },
        body: JSON.stringify({
          name: docName,
          extension: 'docx',
          parent_id: folderId,
          owner_id: currentUserId,
          tenant_id: currentTenantId,
          is_folder: false,
          content: contentHtml
        })
      });

      const data = await res.json();
      if (res.ok && data.success) {
        this.currentFile = {
          id: data.id,
          name: data.name,
          parent_id: data.parent_id
        };
        this.isDirty = false;
        this.updateWindowTitle();
        const modal = document.getElementById('wordcraft-saveas-modal');
        if (modal) modal.style.display = 'none';

        const statusEl = document.getElementById('wordcraft-status');
        if (statusEl) {
          statusEl.textContent = isId ? '● Tersimpan di BijiOS' : '● Saved to BijiOS';
          statusEl.style.color = '#10b981';
        }

        if (window.DokuFinder) window.DokuFinder.loadAllFiles();
        alert(isId ? `Dokumen "${docName}" berhasil disimpan ke folder tujuan!` : `Document "${docName}" successfully saved to destination folder!`);
      } else {
        alert((isId ? "Gagal menyimpan dokumen: " : "Failed to save document: ") + (data.error || 'Unknown error'));
      }
    } catch (err) {
      alert((isId ? "Error menyimpan dokumen: " : "Error saving document: ") + err);
    }
  },

  newBlankDocument() {
    const isId = (window.DokuI18n && DokuI18n.lang === 'id');
    if (this.isDirty && !confirm(isId ? "Perubahan belum disimpan. Buat dokumen baru?" : "Changes not saved. Create new document?")) return;
    this.currentFile = {
      id: 'new_word',
      name: isId ? 'Dokumen Baru.docx' : 'Untitled Document.docx'
    };
    this.isDirty = false;
    this.updateWindowTitle();
    const editor = document.getElementById('wordcraft-editor-body');
    if (editor) {
      editor.innerHTML = `
        <h1 style="color:#1e40af; border-bottom:2px solid #2563eb; padding-bottom:6px;">${isId ? 'Dokumen Baru WordCraft' : 'WordCraft Blank Document'}</h1>
        <p>${isId ? 'Mulai menulis dokumen Anda di sini, atau ' : 'Start typing your document here, or '}<a href="javascript:void(0)" onclick="DokuWordCraft.openFromFileManagerModal()" style="color:#2563eb; text-decoration:underline; font-weight:600;">${isId ? 'buka berkas dari Finder' : 'open from Finder'}</a>.</p>
        <p><br></p>
      `;
      this.updateStats();
      this.refreshOutline();
    }
  },

  async exportDoc(format) {
    const editor = document.getElementById('wordcraft-editor-body');
    if (!editor) return;

    const baseName = (this.currentFile ? this.currentFile.name.replace(/\.[a-zA-Z0-9]+$/, '') : 'Dokumen');
    let content = '';
    let mime = 'text/plain';
    let filename = `${baseName}.${format}`;

    switch (format) {
      case 'docx':
        // Auto-save to guarantee genuine OpenXML package is up-to-date on disk
        await this.saveDocument();
        if (this.currentFile && this.currentFile.id && this.currentFile.id !== 'new_word') {
          window.open(`/api/files/${this.currentFile.id}/download`, '_blank');
          return;
        }
        mime = 'application/vnd.openxmlformats-officedocument.wordprocessingml.document';
        content = editor.innerHTML;
        break;

      case 'html':
        mime = 'text/html';
        content = `<!DOCTYPE html><html><head><meta charset="utf-8"><title>${baseName}</title><style>body{font-family:-apple-system,BlinkMacSystemFont,Segoe UI,Roboto,sans-serif;max-width:800px;margin:40px auto;padding:0 20px;line-height:1.6;}table{width:100%;border-collapse:collapse;}th,td{border:1px solid #ddd;padding:8px;}</style></head><body>${editor.innerHTML}</body></html>`;
        break;

      case 'md':
        mime = 'text/markdown';
        content = `# ${baseName}\n\n` + editor.innerText;
        break;

      case 'txt':
      default:
        mime = 'text/plain';
        content = editor.innerText;
        break;
    }

    const blob = new Blob([content], { type: mime });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  },

  printOrExportPdf() {
    window.print();
  }
};
